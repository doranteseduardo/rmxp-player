use crate::RubyValue;
use anyhow::{anyhow, Context, Result};

#[derive(Debug, Clone)]
pub struct SystemData {
    pub game_title: String,
    pub start_map_id: i32,
    pub start_x: i32,
    pub start_y: i32,
    pub party_members: Vec<i32>,
}

#[derive(Debug, Clone)]
pub struct MapInfoEntry {
    pub id: i32,
    pub name: String,
    pub parent_id: i32,
    pub order: i32,
}

#[derive(Debug, Clone)]
pub struct TableData {
    pub xsize: usize,
    pub ysize: usize,
    pub zsize: usize,
    pub values: Vec<i16>,
}

#[derive(Debug, Clone)]
pub struct MapData {
    pub width: i32,
    pub height: i32,
    pub tileset_id: i32,
    pub data: TableData,
}

pub fn parse_system(value: &RubyValue) -> Result<SystemData> {
    let obj = match value {
        RubyValue::Object(obj) => obj,
        _ => return Err(anyhow!("System value is not an object")),
    };

    let game_title = obj
        .get_string("game_title")
        .unwrap_or_else(|| "Untitled".into());
    let start_map_id = obj.get_int("start_map_id").unwrap_or(1) as i32;
    let start_x = obj.get_int("start_x").unwrap_or(0) as i32;
    let start_y = obj.get_int("start_y").unwrap_or(0) as i32;
    let party_members = extract_int_array(obj.get("party_members")).unwrap_or_default();

    Ok(SystemData {
        game_title,
        start_map_id,
        start_x,
        start_y,
        party_members,
    })
}

pub fn parse_map_infos(value: &RubyValue) -> Result<Vec<MapInfoEntry>> {
    let pairs = match value {
        RubyValue::Hash(pairs) => pairs,
        _ => return Err(anyhow!("MapInfos is not a hash")),
    };

    let mut entries = Vec::with_capacity(pairs.len());
    for (key, entry) in pairs {
        let id = value_to_i32(key).context("MapInfos key is not an integer")?;
        let obj = match entry {
            RubyValue::Object(obj) => obj,
            _ => continue,
        };

        let name = obj
            .get_string("name")
            .unwrap_or_else(|| format!("Map {}", id));
        let parent_id = obj.get_int("parent_id").unwrap_or(0) as i32;
        let order = obj.get_int("order").unwrap_or(0) as i32;

        entries.push(MapInfoEntry {
            id,
            name,
            parent_id,
            order,
        });
    }

    entries.sort_by_key(|entry| entry.order);
    Ok(entries)
}

pub fn parse_map(value: &RubyValue) -> Result<MapData> {
    let obj = match value {
        RubyValue::Object(obj) => obj,
        _ => return Err(anyhow!("map is not an object")),
    };

    let width = obj.get_int("width").unwrap_or(0) as i32;
    let height = obj.get_int("height").unwrap_or(0) as i32;
    let tileset_id = obj.get_int("tileset_id").unwrap_or(1) as i32;
    let data_value = obj
        .get("data")
        .ok_or_else(|| anyhow!("map data missing @data table"))?;
    let data = parse_table(data_value).context("parsing map table")?;

    Ok(MapData {
        width,
        height,
        tileset_id,
        data,
    })
}

impl TableData {
    pub fn new(xsize: usize, ysize: usize, zsize: usize, values: Vec<i16>) -> Self {
        Self {
            xsize,
            ysize,
            zsize,
            values,
        }
    }

    pub fn get(&self, x: usize, y: usize, z: usize) -> Option<i16> {
        if x >= self.xsize || y >= self.ysize || z >= self.zsize {
            return None;
        }
        let idx = x + y * self.xsize + z * self.xsize * self.ysize;
        self.values.get(idx).copied()
    }

    pub fn plane(&self, z: usize) -> Option<Vec<i16>> {
        if z >= self.zsize {
            return None;
        }
        let layer_len = self.xsize * self.ysize;
        let start = z * layer_len;
        let end = start + layer_len;
        Some(self.values[start..end].to_vec())
    }
}

fn value_to_i32(value: &RubyValue) -> Option<i32> {
    match value {
        RubyValue::Integer(v) => (*v).try_into().ok(),
        RubyValue::String(s) => s.to_string_lossy().parse::<i32>().ok(),
        RubyValue::Symbol(sym) => sym.parse::<i32>().ok(),
        _ => None,
    }
}

fn extract_int_array(value: Option<&RubyValue>) -> Option<Vec<i32>> {
    let RubyValue::Array(items) = value? else {
        return None;
    };
    Some(
        items
            .iter()
            .filter_map(|v| value_to_i32(v))
            .collect::<Vec<_>>(),
    )
}

fn parse_table(value: &RubyValue) -> Result<TableData> {
    match value {
        RubyValue::UserDefined { class_name, data } if class_name == "Table" => {
            parse_table_bytes(data)
        }
        _ => Err(anyhow!("expected Table user-defined value")),
    }
}

fn parse_table_bytes(bytes: &[u8]) -> Result<TableData> {
    if bytes.len() < 20 {
        return Err(anyhow!("table data too short"));
    }
    let _dims = i32::from_le_bytes(bytes[0..4].try_into().unwrap());
    let xsize = i32::from_le_bytes(bytes[4..8].try_into().unwrap()).max(0) as usize;
    let ysize = i32::from_le_bytes(bytes[8..12].try_into().unwrap()).max(0) as usize;
    let zsize = i32::from_le_bytes(bytes[12..16].try_into().unwrap()).max(0) as usize;
    let len = i32::from_le_bytes(bytes[16..20].try_into().unwrap()).max(0) as usize;
    let mut values = Vec::with_capacity(len);
    let mut offset = 20;
    while offset + 2 <= bytes.len() && values.len() < len {
        let value = i16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap());
        values.push(value);
        offset += 2;
    }
    if values.len() < len {
        values.resize(len, 0);
    }
    Ok(TableData::new(xsize, ysize, zsize.max(1), values))
}

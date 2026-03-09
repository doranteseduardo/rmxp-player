// Helper to manage RMXP Table data structure from JSON
export class Table {
  xSize: number;
  ySize: number;
  zSize: number;
  data: number[];

  constructor(json: any) {
    if (json && json.__class === "Table") {
      this.xSize = json.x_size || 0;
      this.ySize = json.y_size || 0;
      this.zSize = json.z_size || 0;
      this.data = json.data || [];
    } else {
      this.xSize = 0;
      this.ySize = 0;
      this.zSize = 0;
      this.data = [];
    }
  }

  get(x: number, y: number, z: number = 0): number {
    if (
      x < 0 ||
      x >= this.xSize ||
      y < 0 ||
      y >= this.ySize ||
      z < 0 ||
      z >= this.zSize
    ) {
      return 0;
    }
    // RMXP Table index: x + (y * x_size) + (z * x_size * y_size)
    const index = x + y * this.xSize + z * this.xSize * this.ySize;
    return this.data[index] || 0;
  }
}

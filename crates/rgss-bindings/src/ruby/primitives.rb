# Minimal RGSS class surface to unblock scripts until native bindings land.
module RGSS
  module Debug
    @warned = {}

    def self.warn_once(label)
      key = label.to_s
      return if @warned[key]
      warn("[RGSS] #{label} is not implemented yet")
      @warned[key] = true
    end
  end
end

class Color
  attr_accessor :red, :green, :blue, :alpha

  def initialize(red = 0, green = 0, blue = 0, alpha = 255)
    set(red, green, blue, alpha)
  end

  def set(red = 0, green = 0, blue = 0, alpha = 255)
    @red = red.to_f
    @green = green.to_f
    @blue = blue.to_f
    @alpha = alpha.to_f
    self
  end

  def ==(other)
    other.is_a?(Color) &&
      @red == other.red && @green == other.green &&
      @blue == other.blue && @alpha == other.alpha
  end

  def dup
    Color.new(@red, @green, @blue, @alpha)
  end
end

class Tone
  attr_accessor :red, :green, :blue, :gray

  def initialize(red = 0, green = 0, blue = 0, gray = 0)
    set(red, green, blue, gray)
  end

  def set(red = 0, green = 0, blue = 0, gray = 0)
    @red = red.to_f
    @green = green.to_f
    @blue = blue.to_f
    @gray = gray.to_f
    self
  end

  def ==(other)
    other.is_a?(Tone) &&
      @red == other.red && @green == other.green &&
      @blue == other.blue && @gray == other.gray
  end

  def dup
    Tone.new(@red, @green, @blue, @gray)
  end
end

class Rect
  attr_accessor :x, :y, :width, :height

  def initialize(x = 0, y = 0, width = 0, height = 0)
    set(x, y, width, height)
  end

  def set(x = 0, y = 0, width = 0, height = 0)
    @x = x.to_i
    @y = y.to_i
    @width = width.to_i
    @height = height.to_i
    self
  end

  def empty
    set(0, 0, 0, 0)
  end

  def dup
    Rect.new(@x, @y, @width, @height)
  end

  def ==(other)
    other.is_a?(Rect) &&
      @x == other.x && @y == other.y &&
      @width == other.width && @height == other.height
  end
end

class Table
  attr_reader :xsize, :ysize, :zsize

  def initialize(x = 0, y = 0, z = 0)
    resize(x, y, z)
  end

  def resize(x, y = 1, z = 1)
    @xsize = [x.to_i, 0].max
    @ysize = [y.to_i, 1].max
    @zsize = [z.to_i, 1].max
    @data = Array.new(@xsize * @ysize * @zsize, 0)
  end

  def [](x, y = 0, z = 0)
    idx = index_of(x, y, z)
    idx ? @data[idx] : 0
  end

  def []=(x, y = 0, z = 0, value)
    idx = index_of(x, y, z)
    @data[idx] = value.to_i if idx
  end

  def clone
    other = Table.new(@xsize, @ysize, @zsize)
    other.instance_variable_set(:@data, @data.dup)
    other
  end

  alias dup clone

  private

  def index_of(x, y, z)
    xi = x.to_i
    yi = y.to_i
    zi = z.to_i
    return nil if xi < 0 || yi < 0 || zi < 0
    return nil if xi >= @xsize || yi >= @ysize || zi >= @zsize
    xi + yi * @xsize + zi * @xsize * @ysize
  end
end

module Cache
  @cache = {}

  def self.load_bitmap(folder_name, filename, hue = 0)
    path = folder_name + filename
    if !@cache.include?(path) || @cache[path].disposed?
      if filename != ""
        @cache[path] = Bitmap.new(path)
      else
        @cache[path] = Bitmap.new(32, 32)
      end
    end
    if hue == 0
      @cache[path]
    else
      key = [path, hue]
      if !@cache.include?(key) || @cache[key].disposed?
        @cache[key] = @cache[path].dup
        @cache[key].hue_change(hue)
      end
      @cache[key]
    end
  end

  def self.animation(filename, hue)
    load_bitmap("Graphics/Animations/", filename, hue)
  end

  def self.autotile(filename)
    load_bitmap("Graphics/Autotiles/", filename)
  end

  def self.battleback(filename)
    load_bitmap("Graphics/Battlebacks/", filename)
  end

  def self.battler(filename, hue)
    load_bitmap("Graphics/Battlers/", filename, hue)
  end

  def self.character(filename, hue)
    load_bitmap("Graphics/Characters/", filename, hue)
  end

  def self.fog(filename, hue)
    load_bitmap("Graphics/Fogs/", filename, hue)
  end

  def self.gameover(filename)
    load_bitmap("Graphics/Gameovers/", filename)
  end

  def self.icon(filename)
    load_bitmap("Graphics/Icons/", filename)
  end

  def self.panorama(filename, hue = 0)
    load_bitmap("Graphics/Panoramas/", filename, hue)
  end

  def self.picture(filename)
    load_bitmap("Graphics/Pictures/", filename)
  end

  def self.tileset(filename)
    load_bitmap("Graphics/Tilesets/", filename)
  end

  def self.title(filename)
    load_bitmap("Graphics/Titles/", filename)
  end

  def self.windowskin(filename)
    load_bitmap("Graphics/Windowskins/", filename)
  end
end

class Font
  attr_accessor :name, :size, :bold, :italic, :shadow, :color
  @@default_name = ["Arial"]
  @@default_size = 24
  @@default_bold = false
  @@default_italic = false
  @@default_shadow = false
  @@default_color = Color.new(255, 255, 255, 255)

  def self.default_name
    @@default_name
  end

  def self.default_name=(value)
    @@default_name = Array(value).map(&:to_s)
  end

  def self.default_size
    @@default_size
  end

  def self.default_size=(value)
    @@default_size = value.to_i
  end

  def self.default_bold
    @@default_bold
  end

  def self.default_bold=(value)
    @@default_bold = !!value
  end

  def self.default_italic
    @@default_italic
  end

  def self.default_italic=(value)
    @@default_italic = !!value
  end

  def self.default_shadow
    @@default_shadow
  end

  def self.default_shadow=(value)
    @@default_shadow = !!value
  end

  def self.default_color
    @@default_color
  end

  def self.default_color=(value)
    @@default_color = value.is_a?(Color) ? value : Color.new
  end

  def initialize(name = nil, size = nil)
    @name = name ? Array(name).map(&:to_s) : @@default_name.dup
    @size = (size || @@default_size).to_i
    @bold = @@default_bold
    @italic = @@default_italic
    @shadow = @@default_shadow
    @color = @@default_color.dup
  end
end

class Bitmap
  attr_reader :width, :height
  attr_accessor :font

  def initialize(arg1, arg2 = nil)
    if arg1.is_a?(String)
      @path = arg1
      @width = arg2 ? arg2.to_i : 0
      @height = 0
    else
      @path = nil
      @width = arg1.to_i
      @height = (arg2 || arg1).to_i
    end
    @disposed = false
    @font = Font.new
  end

  def rect
    Rect.new(0, 0, @width, @height)
  end

  def disposed?
    @disposed
  end

  def dispose
    @disposed = true
  end

  def hue_change(_value)
    RGSS::Debug.warn_once('Bitmap#hue_change')
  end

  def blt(*_args)
    RGSS::Debug.warn_once('Bitmap#blt')
  end

  def stretch_blt(*_args)
    RGSS::Debug.warn_once('Bitmap#stretch_blt')
  end

  def fill_rect(*_args)
    RGSS::Debug.warn_once('Bitmap#fill_rect')
  end

  def gradient_fill_rect(*_args)
    RGSS::Debug.warn_once('Bitmap#gradient_fill_rect')
  end

  def clear
    RGSS::Debug.warn_once('Bitmap#clear')
  end

  def text_size(_text)
    Rect.new(0, 0, 0, 0)
  end

  def draw_text(*_args)
    RGSS::Debug.warn_once('Bitmap#draw_text')
  end

  def get_pixel(_x, _y)
    Color.new
  end

  def set_pixel(*_args)
    RGSS::Debug.warn_once('Bitmap#set_pixel')
  end

  def dup
    copy = Bitmap.new(@width, @height)
    RGSS::Debug.warn_once('Bitmap#dup')
    copy
  end
end

class Viewport
  attr_accessor :rect, :visible, :z, :ox, :oy, :color, :tone

  def initialize(x_or_rect, y = nil, width = nil, height = nil)
    if x_or_rect.is_a?(Rect)
      @rect = x_or_rect.dup
    else
      @rect = Rect.new(x_or_rect, y || 0, width || 0, height || 0)
    end
    @visible = true
    @z = 0
    @ox = 0
    @oy = 0
    @color = Color.new
    @tone = Tone.new
    @disposed = false
  end

  def update; end

  def disposed?
    @disposed
  end

  def dispose
    @disposed = true
  end
end

class Sprite
  attr_accessor :bitmap, :x, :y, :z, :ox, :oy, :zoom_x, :zoom_y,
                :angle, :mirror, :bush_depth, :opacity, :blend_type,
                :color, :tone, :visible, :viewport, :src_rect

  def initialize(viewport = nil)
    @viewport = viewport
    @bitmap = nil
    @x = @y = 0
    @z = 0
    @ox = @oy = 0
    @zoom_x = @zoom_y = 1.0
    @angle = 0.0
    @mirror = false
    @bush_depth = 0
    @opacity = 255
    @blend_type = 0
    @color = Color.new
    @tone = Tone.new
    @visible = true
    @src_rect = Rect.new
    @disposed = false
  end

  def disposed?
    @disposed
  end

  def dispose
    @disposed = true
  end

  def flash(*_args)
    RGSS::Debug.warn_once('Sprite#flash')
  end

  def update; end
end

class Plane < Sprite
end

class Window
  attr_accessor :x, :y, :z, :ox, :oy, :width, :height, :visible,
                :openness, :windowskin, :contents, :active, :pause,
                :cursor_rect, :tone, :color, :opacity, :back_opacity,
                :contents_opacity, :viewport

  def initialize(x = 0, y = 0, width = 32, height = 32, viewport = nil)
    @x = x.to_i
    @y = y.to_i
    @z = 0
    @ox = @oy = 0
    @width = width.to_i
    @height = height.to_i
    @visible = true
    @openness = 255
    @windowskin = nil
    @contents = nil
    @active = true
    @pause = false
    @cursor_rect = Rect.new
    @tone = Tone.new
    @color = Color.new
    @opacity = 255
    @back_opacity = 255
    @contents_opacity = 255
    @viewport = viewport
    @disposed = false
  end

  def disposed?
    @disposed
  end

  def dispose
    @disposed = true
  end

  def open
    @openness = 255
  end

  def close
    @openness = 0
  end

  def update; end
end

class Tilemap
  attr_accessor :viewport, :bitmaps, :map_data, :flash_data,
                :ox, :oy, :tileset, :autotiles, :visible, :priorities

  def initialize(viewport = nil)
    @viewport = viewport
    @tileset = nil
    @autotiles = Array.new(7)
    @bitmaps = Array.new(9)
    @map_data = nil
    @flash_data = nil
    @priorities = Table.new
    @ox = 0
    @oy = 0
    @visible = true
    @disposed = false
  end

  def disposed?
    @disposed
  end

  def dispose
    @disposed = true
  end

  def update; end
end

module Audio
  module_function

  def bgm_play(*_args)
    RGSS::Debug.warn_once('Audio.bgm_play')
  end

  def bgm_stop
    RGSS::Debug.warn_once('Audio.bgm_stop')
  end

  def bgm_fade(_time)
    RGSS::Debug.warn_once('Audio.bgm_fade')
  end

  def bgs_play(*_args)
    RGSS::Debug.warn_once('Audio.bgs_play')
  end

  def bgs_stop
    RGSS::Debug.warn_once('Audio.bgs_stop')
  end

  def bgs_fade(_time)
    RGSS::Debug.warn_once('Audio.bgs_fade')
  end

  def me_play(*_args)
    RGSS::Debug.warn_once('Audio.me_play')
  end

  def me_stop
    RGSS::Debug.warn_once('Audio.me_stop')
  end

  def me_fade(_time)
    RGSS::Debug.warn_once('Audio.me_fade')
  end

  def se_play(*_args)
    RGSS::Debug.warn_once('Audio.se_play')
  end

  def se_stop
    RGSS::Debug.warn_once('Audio.se_stop')
  end

  def se_fade(_time)
    RGSS::Debug.warn_once('Audio.se_fade')
  end
end

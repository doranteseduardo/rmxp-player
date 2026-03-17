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

begin
  require 'fileutils'
rescue LoadError
  module FileUtils
    module_function

    def mkdir_p(path)
      return if path.nil? || path.empty?
      segments = []
      path_parts = path.split(File::SEPARATOR)
      base = path.start_with?(File::SEPARATOR) ? File::SEPARATOR : ""
      path_parts.each do |segment|
        next if segment.nil? || segment.empty?
        base = base == File::SEPARATOR ? File.join(base, segment) : File.join(base, segment)
        next if Dir.exist?(base)
        Dir.mkdir(base)
      end
    rescue SystemCallError
      raise
    end
  end
end

module RGSS
  module Runtime
    class << self
      attr_reader :main_fiber

      def install_main(&block)
        raise ArgumentError, 'rgss_main requires a block' unless block
        @main_fiber = Fiber.new do
          begin
            block.call
          ensure
            @main_fiber = nil
          end
        end
      end

      def resume_main
        fiber = @main_fiber
        return false unless fiber
        if fiber.alive?
          fiber.resume
          true
        else
          @main_fiber = nil
          false
        end
      rescue Exception
        @main_fiber = nil
        raise
      end

      def yield_frame
        fiber = @main_fiber
        return unless fiber
        current = Fiber.current
        return unless current.equal?(fiber)
        Fiber.yield
      end

      def active?
        fiber = @main_fiber
        fiber && fiber.alive?
      end

      def reset
        @main_fiber = nil
      end
    end
  end
end

class Hangup < Exception; end

unless Numeric.method_defined?(:to_i)
  class Numeric
    def to_i
      return self if is_a?(Integer)
      self < 0 ? self.ceil : self.floor
    end
  end
end

unless String.method_defined?(:message)
  class String
    def message
      self
    end
  end
end

module Kernel
  def data_path(relative = '')
    RGSS::Native.project_path(relative.to_s)
  end

  def load_data(filename)
    path = resolve_rgss_read_path(filename)
    File.open(path, 'rb') { |f| Marshal.load(f) }
  end

  def save_data(object, filename)
    path = resolve_rgss_write_path(filename)
    File.open(path, 'wb') { |f| Marshal.dump(object, f) }
    object
  end

  def data_exist?(filename)
    path = RGSS::Native.project_path(filename.to_s)
    path && File.exist?(path)
  end

  private

  def resolve_rgss_read_path(filename)
    name = filename.to_s
    path = RGSS::Native.project_path(name)
    raise Errno::ENOENT, name unless path && File.exist?(path)
    path
  end

  def resolve_rgss_write_path(filename)
    name = filename.to_s
    path = RGSS::Native.project_path(name)
    raise Errno::ENOENT, name unless path
    dir = File.dirname(path)
    FileUtils.mkdir_p(dir) if dir && !dir.empty?
    path
  end

  if method_defined?(:rgss_stop) && !method_defined?(:__rgss_native_stop)
    alias_method :__rgss_native_stop, :rgss_stop
  end

  def rgss_main(&block)
    unless block
      RGSS::Debug.warn_once('rgss_main requires a block')
      return
    end
    RGSS::Runtime.install_main(&block)
  end

  def rgss_stop
    RGSS::Runtime.reset
    __rgss_native_stop if defined?(__rgss_native_stop)
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

  def to_native_s16
    @data.pack('s<*')
  end

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
    @@default_color = value.is_a?(Color) ? value : Color.new(255, 255, 255, 255)
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
  attr_accessor :font
  attr_reader :native_id

  @max_size = 16_384

  class << self
    def max_size(value = nil)
      @max_size = value.to_i if value
      @max_size
    end

    def max_size=(value)
      @max_size = value.to_i
    end
  end

  class << self
    def _native_wrap(handle)
      return nil unless handle
      obj = allocate
      obj.instance_variable_set(:@font, Font.new)
      obj.instance_variable_set(:@native_id, handle)
      obj
    end
  end

  def initialize(arg1, arg2 = nil)
    @font = Font.new
    if arg1.is_a?(String)
      @path = arg1
      @native_id = RGSS::Native.bitmap_load(@path)
      unless @native_id
        RGSS::Debug.warn_once("Bitmap load failed: #{@path}")
        allocate_blank(32, 32)
      end
    else
      width = arg1.to_i
      height = (arg2 || arg1).to_i
      allocate_blank(width, height)
    end
  end

  def rect
    Rect.new(0, 0, width, height)
  end

  def disposed?
    return true unless @native_id
    RGSS::Native.bitmap_disposed?(@native_id)
  end

  def dispose
    return unless @native_id
    RGSS::Native.bitmap_dispose(@native_id)
    @native_id = nil
  end

  def width
    return 0 unless @native_id
    RGSS::Native.bitmap_width(@native_id)
  end

  def height
    return 0 unless @native_id
    RGSS::Native.bitmap_height(@native_id)
  end

  def hue_change(_value)
    RGSS::Debug.warn_once('Bitmap#hue_change')
  end

  def blt(x, y, src_bitmap, src_rect, opacity = 255)
    return unless @native_id && src_bitmap&.native_id && src_rect
    RGSS::Native.bitmap_blt(
      @native_id,
      x.to_i,
      y.to_i,
      src_bitmap.native_id,
      src_rect.x,
      src_rect.y,
      src_rect.width,
      src_rect.height,
      opacity.to_i
    )
  end

  def stretch_blt(*args)
    rect, src_bitmap, src_rect, opacity = normalize_stretch_args(args)
    return unless @native_id && src_bitmap&.native_id && src_rect
    RGSS::Native.bitmap_stretch_blt(
      @native_id,
      rect.x,
      rect.y,
      rect.width,
      rect.height,
      src_bitmap.native_id,
      src_rect.x,
      src_rect.y,
      src_rect.width,
      src_rect.height,
      opacity.to_i
    )
  end

  def fill_rect(*args)
    rect, color = normalize_rect_color_args(args)
    return unless rect && color && @native_id
    RGSS::Native.bitmap_fill_rect(
      @native_id,
      rect.x,
      rect.y,
      rect.width,
      rect.height,
      pack_color(color)
    )
  end

  def gradient_fill_rect(*_args)
    rect, c1, c2, vertical = normalize_gradient_args(_args)
    return unless @native_id && c1 && c2
    RGSS::Native.bitmap_gradient_fill_rect(
      @native_id,
      rect.x,
      rect.y,
      rect.width,
      rect.height,
      pack_color(c1),
      pack_color(c2),
      !!vertical
    )
  end

  def clear
    RGSS::Native.bitmap_clear(@native_id) if @native_id
  end

  def text_size(_text)
    return Rect.new unless @native_id
    result = RGSS::Native.bitmap_text_size(@font.size, _text.to_s)
    Rect.new(0, 0, result[0].to_i, result[1].to_i)
  end

  def draw_text(*args)
    rect, text, align = normalize_draw_text_args(args)
    return unless @native_id
    RGSS::Native.bitmap_draw_text(
      @native_id,
      rect.x,
      rect.y,
      rect.width,
      rect.height,
      text.to_s,
      align.to_i,
      @font.size,
      pack_color(@font.color)
    )
  end

  def get_pixel(x, y)
    return Color.new(0, 0, 0, 0) unless @native_id
    packed = RGSS::Native.bitmap_get_pixel(@native_id, x.to_i, y.to_i)
    return Color.new(0, 0, 0, 0) unless packed
    r = packed & 0xFF
    g = (packed >> 8) & 0xFF
    b = (packed >> 16) & 0xFF
    a = (packed >> 24) & 0xFF
    Color.new(r, g, b, a)
  end

  def set_pixel(x, y, color)
    return unless @native_id && color
    RGSS::Native.bitmap_set_pixel(
      @native_id,
      x.to_i,
      y.to_i,
      color.red.to_i,
      color.green.to_i,
      color.blue.to_i,
      color.alpha.to_i
    )
  end

  def dup
    copy = Bitmap.new(width, height)
    copy.font = @font.dup
    copy_rect = Rect.new(0, 0, width, height)
    copy.blt(0, 0, self, copy_rect)
    copy
  end

  private

  def normalize_rect_color_args(args)
    if args.length == 2 && args.first.is_a?(Rect)
      [args.first, args.last]
    elsif args.length >= 5
      rect = Rect.new(args[0], args[1], args[2], args[3])
      [rect, args[4]]
    else
      [nil, nil]
    end
  end

  def normalize_stretch_args(args)
    if args.length >= 3 && args[0].is_a?(Rect)
      rect = args[0].dup
      src_bitmap = args[1]
      src_rect = args[2]
      opacity = args[3] || 255
    else
      rect = Rect.new(args[0], args[1], args[2], args[3])
      src_bitmap = args[4]
      src_rect = args[5]
      opacity = args[6] || 255
    end
    src_rect ||= src_bitmap&.rect
    [rect, src_bitmap, src_rect, opacity]
  end

  def normalize_gradient_args(args)
    if args.length >= 3 && args[0].is_a?(Rect)
      rect = args[0].dup
      color1 = args[1]
      color2 = args[2]
      vertical = args[3]
    else
      rect = Rect.new(args[0], args[1], args[2], args[3])
      color1 = args[4]
      color2 = args[5]
      vertical = args[6]
    end
    [rect, ensure_color(color1), ensure_color(color2), vertical]
  end

  def normalize_draw_text_args(args)
    if args.length >= 2 && args[0].is_a?(Rect)
      rect = args[0].dup
      text = args[1]
      align = args[2] || 0
    else
      rect = Rect.new(args[0], args[1], args[2], args[3])
      text = args[4]
      align = args[5] || 0
    end
    [rect, text, align]
  end

  def ensure_color(value)
    value.is_a?(Color) ? value : Color.new(0, 0, 0, 0)
  end

  def pack_color(color)
    r = color.red.to_i.clamp(0, 255)
    g = color.green.to_i.clamp(0, 255)
    b = color.blue.to_i.clamp(0, 255)
    a = color.alpha.to_i.clamp(0, 255)
    r | (g << 8) | (b << 16) | (a << 24)
  end

  def allocate_blank(width, height)
    unless RGSS.const_defined?(:Native)
      RGSS::Debug.warn_once('RGSS::Native bitmap bridge not available')
      @native_id = nil
      return
    end
    @native_id = RGSS::Native.bitmap_create([width, 1].max, [height, 1].max)
  end
end

module Graphics
  class << self
    alias_method :__snap_to_bitmap_handle, :_snap_to_bitmap_handle

    def snap_to_bitmap
      handle = __snap_to_bitmap_handle()
      return nil unless handle
      Bitmap._native_wrap(handle)
    end

    def tone
      components = _tone_vector
      (@tone ||= Tone.new).set(*components)
    end

    def tone=(value)
      tone = value.is_a?(Tone) ? value : Tone.new
      _set_tone(tone.red, tone.green, tone.blue, tone.gray)
    end

    def brightness
      _brightness_value
    end

    def brightness=(value)
      _set_brightness(value.to_i)
    end

    def flash(color = nil, duration = 0)
      color ||= Color.new(255, 255, 255, 255)
      _flash(color.red, color.green, color.blue, color.alpha, duration.to_i)
    end
  end
end

class Viewport
  attr_reader :rect, :visible, :z, :ox, :oy, :color, :tone, :native_id

  def initialize(x_or_rect, y = nil, width = nil, height = nil)
    rect = x_or_rect.is_a?(Rect) ? x_or_rect.dup : Rect.new(x_or_rect, y || 0, width || 0, height || 0)
    @rect = rect
    @visible = true
    @z = 0
    @ox = 0
    @oy = 0
    @color = Color.new(0, 0, 0, 0)
    @tone = Tone.new
    @disposed = false
    @native_id = RGSS::Native.viewport_create(@rect.x, @rect.y, @rect.width, @rect.height)
    sync_rect
    sync_visible
    sync_z
    sync_color
    sync_tone
    sync_origin
  end

  def update; end

  def disposed?
    @disposed
  end

  def dispose
    return if disposed?
    RGSS::Native.viewport_dispose(@native_id)
    @disposed = true
  end

  def rect=(rect)
    @rect = rect.dup
    sync_rect
  end

  def visible=(value)
    @visible = !!value
    sync_visible
  end

  def z=(value)
    @z = value.to_i
    sync_z
  end

  def color=(value)
    @color = value.is_a?(Color) ? value.dup : Color.new(0, 0, 0, 0)
    sync_color
  end

  def tone=(value)
    @tone = value.is_a?(Tone) ? value.dup : Tone.new
    sync_tone
  end

  def ox=(value)
    @ox = value.to_i
    RGSS::Native.viewport_set_ox(@native_id, @ox)
  end

  def oy=(value)
    @oy = value.to_i
    RGSS::Native.viewport_set_oy(@native_id, @oy)
  end

  private

  def sync_rect
    RGSS::Native.viewport_set_rect(@native_id, @rect.x, @rect.y, @rect.width, @rect.height)
  end

  def sync_visible
    RGSS::Native.viewport_set_visible(@native_id, @visible)
  end

  def sync_z
    RGSS::Native.viewport_set_z(@native_id, @z)
  end

  def sync_color
    RGSS::Native.viewport_set_color(@native_id, @color.red, @color.green, @color.blue, @color.alpha)
  end

  def sync_tone
    RGSS::Native.viewport_set_tone(@native_id, @tone.red, @tone.green, @tone.blue, @tone.gray)
  end

  def sync_origin
    RGSS::Native.viewport_set_ox(@native_id, @ox)
    RGSS::Native.viewport_set_oy(@native_id, @oy)
  end
end

class Sprite
  attr_reader :bitmap, :viewport, :color, :tone, :src_rect, :native_id
  attr_reader :x, :y, :z, :ox, :oy, :zoom_x, :zoom_y, :angle, :mirror,
              :bush_depth, :bush_opacity, :opacity, :blend_type, :visible

  def initialize(viewport = nil)
    @viewport = viewport
    @bitmap = nil
    @x = @y = 0.0
    @z = 0
    @ox = @oy = 0.0
    @zoom_x = @zoom_y = 1.0
    @angle = 0.0
    @mirror = false
    @bush_depth = 0
    @bush_opacity = 128
    @opacity = 255
    @blend_type = 0
    @visible = true
    @color = Color.new(0, 0, 0, 0)
    @tone = Tone.new
    @src_rect = Rect.new
    @disposed = false
    @native_id = RGSS::Native.sprite_create(@viewport&.native_id)
    sync_all
  end

  def disposed?
    @disposed
  end

  def dispose
    return if disposed?
    RGSS::Native.sprite_dispose(@native_id)
    @disposed = true
  end

  def viewport=(viewport)
    @viewport = viewport
    RGSS::Native.sprite_set_viewport(@native_id, @viewport&.native_id)
  end

  def bitmap=(bitmap)
    @bitmap = bitmap
    RGSS::Native.sprite_set_bitmap(@native_id, @bitmap&.native_id)
  end

  def width
    return @src_rect.width if @bitmap.nil?
    @bitmap.width
  end

  def height
    return @src_rect.height if @bitmap.nil?
    @bitmap.height
  end

  def x=(value)
    @x = value.to_f
    RGSS::Native.sprite_set_x(@native_id, @x)
  end

  def y=(value)
    @y = value.to_f
    RGSS::Native.sprite_set_y(@native_id, @y)
  end

  def z=(value)
    @z = value.to_i
    RGSS::Native.sprite_set_z(@native_id, @z)
  end

  def ox=(value)
    @ox = value.to_f
    RGSS::Native.sprite_set_ox(@native_id, @ox)
  end

  def oy=(value)
    @oy = value.to_f
    RGSS::Native.sprite_set_oy(@native_id, @oy)
  end

  def zoom_x=(value)
    @zoom_x = value.to_f
    RGSS::Native.sprite_set_zoom_x(@native_id, @zoom_x)
  end

  def zoom_y=(value)
    @zoom_y = value.to_f
    RGSS::Native.sprite_set_zoom_y(@native_id, @zoom_y)
  end

  def angle=(value)
    @angle = value.to_f
    RGSS::Native.sprite_set_angle(@native_id, @angle)
  end

  def mirror=(value)
    @mirror = !!value
    RGSS::Native.sprite_set_mirror(@native_id, @mirror)
  end

  def bush_depth=(value)
    @bush_depth = value.to_i
    RGSS::Native.sprite_set_bush_depth(@native_id, @bush_depth)
  end

  def bush_opacity=(value)
    @bush_opacity = value.to_i.clamp(0, 255)
    RGSS::Native.sprite_set_bush_opacity(@native_id, @bush_opacity)
  end

  def opacity=(value)
    @opacity = value.to_i
    RGSS::Native.sprite_set_opacity(@native_id, @opacity)
  end

  def blend_type=(value)
    @blend_type = value.to_i
    RGSS::Native.sprite_set_blend_type(@native_id, @blend_type)
  end

  def visible=(value)
    @visible = !!value
    RGSS::Native.sprite_set_visible(@native_id, @visible)
  end

  def src_rect=(rect)
    @src_rect = rect.dup
    RGSS::Native.sprite_set_src_rect(@native_id, @src_rect.x, @src_rect.y, @src_rect.width, @src_rect.height)
  end

  def color=(value)
    @color = value.is_a?(Color) ? value.dup : Color.new(0, 0, 0, 0)
    RGSS::Native.sprite_set_color(@native_id, @color.red, @color.green, @color.blue, @color.alpha)
  end

  def tone=(value)
    @tone = value.is_a?(Tone) ? value.dup : Tone.new
    RGSS::Native.sprite_set_tone(@native_id, @tone.red, @tone.green, @tone.blue, @tone.gray)
  end

  def flash(*_args)
    RGSS::Debug.warn_once('Sprite#flash')
  end

  def update; end

  private

  def sync_all
    RGSS::Native.sprite_set_viewport(@native_id, @viewport&.native_id)
    RGSS::Native.sprite_set_bitmap(@native_id, @bitmap&.native_id)
    RGSS::Native.sprite_set_x(@native_id, @x)
    RGSS::Native.sprite_set_y(@native_id, @y)
    RGSS::Native.sprite_set_z(@native_id, @z)
    RGSS::Native.sprite_set_ox(@native_id, @ox)
    RGSS::Native.sprite_set_oy(@native_id, @oy)
    RGSS::Native.sprite_set_zoom_x(@native_id, @zoom_x)
    RGSS::Native.sprite_set_zoom_y(@native_id, @zoom_y)
    RGSS::Native.sprite_set_angle(@native_id, @angle)
    RGSS::Native.sprite_set_mirror(@native_id, @mirror)
    RGSS::Native.sprite_set_bush_depth(@native_id, @bush_depth)
    RGSS::Native.sprite_set_bush_opacity(@native_id, @bush_opacity)
    RGSS::Native.sprite_set_opacity(@native_id, @opacity)
    RGSS::Native.sprite_set_blend_type(@native_id, @blend_type)
    RGSS::Native.sprite_set_visible(@native_id, @visible)
    RGSS::Native.sprite_set_src_rect(@native_id, @src_rect.x, @src_rect.y, @src_rect.width, @src_rect.height)
    RGSS::Native.sprite_set_color(@native_id, @color.red, @color.green, @color.blue, @color.alpha)
    RGSS::Native.sprite_set_tone(@native_id, @tone.red, @tone.green, @tone.blue, @tone.gray)
  end
end

class Plane < Sprite
  attr_reader :viewport, :bitmap, :tone, :color, :native_id
  attr_reader :z, :ox, :oy, :zoom_x, :zoom_y, :opacity, :blend_type, :visible

  def initialize(viewport = nil)
    @viewport = viewport
    @bitmap = nil
    @z = 0
    @ox = 0.0
    @oy = 0.0
    @zoom_x = 1.0
    @zoom_y = 1.0
    @opacity = 255
    @blend_type = 0
    @visible = true
    @tone = Tone.new
    @color = Color.new(0, 0, 0, 0)
    @disposed = false
    @native_id = RGSS::Native.plane_create(@viewport&.native_id)
    sync_all
  end

  def disposed?
    @disposed
  end

  def dispose
    return if disposed?
    RGSS::Native.plane_dispose(@native_id)
    @disposed = true
  end

  def viewport=(viewport)
    @viewport = viewport
    RGSS::Native.plane_set_viewport(@native_id, @viewport&.native_id)
  end

  def bitmap=(bitmap)
    @bitmap = bitmap
    RGSS::Native.plane_set_bitmap(@native_id, @bitmap&.native_id)
  end

  def z=(value)
    @z = value.to_i
    RGSS::Native.plane_set_z(@native_id, @z)
  end

  def ox=(value)
    @ox = value.to_f
    RGSS::Native.plane_set_ox(@native_id, @ox)
  end

  def oy=(value)
    @oy = value.to_f
    RGSS::Native.plane_set_oy(@native_id, @oy)
  end

  def zoom_x=(value)
    @zoom_x = value.to_f
    RGSS::Native.plane_set_zoom_x(@native_id, @zoom_x)
  end

  def zoom_y=(value)
    @zoom_y = value.to_f
    RGSS::Native.plane_set_zoom_y(@native_id, @zoom_y)
  end

  def opacity=(value)
    @opacity = value.to_i
    RGSS::Native.plane_set_opacity(@native_id, @opacity)
  end

  def blend_type=(value)
    @blend_type = value.to_i
    RGSS::Native.plane_set_blend_type(@native_id, @blend_type)
  end

  def visible=(value)
    @visible = !!value
    RGSS::Native.plane_set_visible(@native_id, @visible)
  end

  def tone=(value)
    @tone = value.is_a?(Tone) ? value.dup : Tone.new
    RGSS::Native.plane_set_tone(@native_id, @tone.red, @tone.green, @tone.blue, @tone.gray)
  end

  def color=(value)
    @color = value.is_a?(Color) ? value.dup : Color.new(0, 0, 0, 0)
    RGSS::Native.plane_set_color(@native_id, @color.red, @color.green, @color.blue, @color.alpha)
  end

  def update; end

  private

  def sync_all
    RGSS::Native.plane_set_viewport(@native_id, @viewport&.native_id)
    RGSS::Native.plane_set_bitmap(@native_id, @bitmap&.native_id)
    RGSS::Native.plane_set_z(@native_id, @z)
    RGSS::Native.plane_set_ox(@native_id, @ox)
    RGSS::Native.plane_set_oy(@native_id, @oy)
    RGSS::Native.plane_set_zoom_x(@native_id, @zoom_x)
    RGSS::Native.plane_set_zoom_y(@native_id, @zoom_y)
    RGSS::Native.plane_set_opacity(@native_id, @opacity)
    RGSS::Native.plane_set_blend_type(@native_id, @blend_type)
    RGSS::Native.plane_set_visible(@native_id, @visible)
    RGSS::Native.plane_set_tone(@native_id, @tone.red, @tone.green, @tone.blue, @tone.gray)
    RGSS::Native.plane_set_color(@native_id, @color.red, @color.green, @color.blue, @color.alpha)
  end
end

class Window
  attr_reader :viewport, :windowskin, :contents, :cursor_rect, :tone, :color,
              :native_id, :x, :y, :z, :ox, :oy, :width, :height,
              :opacity, :back_opacity, :contents_opacity, :openness,
              :visible, :active, :pause

  def initialize(x = 0, y = 0, width = 32, height = 32, viewport = nil)
    @x = x.to_i
    @y = y.to_i
    @z = 0
    @ox = @oy = 0
    @width = width.to_i
    @height = height.to_i
    @opacity = 255
    @back_opacity = 255
    @contents_opacity = 255
    @openness = 255
    @visible = true
    @active = true
    @pause = false
    @windowskin = nil
    @contents = nil
    @cursor_rect = Rect.new
    @tone = Tone.new
    @color = Color.new(0, 0, 0, 0)
    @viewport = viewport
    @disposed = false
    @native_id = RGSS::Native.window_create
    sync_all
  end

  def disposed?
    @disposed
  end

  def dispose
    return if disposed?
    RGSS::Native.window_dispose(@native_id)
    @disposed = true
  end

  def viewport=(viewport)
    @viewport = viewport
    RGSS::Native.window_set_viewport(@native_id, @viewport&.native_id)
  end

  def windowskin=(bitmap)
    @windowskin = bitmap
    RGSS::Native.window_set_windowskin(@native_id, @windowskin&.native_id)
  end

  def contents=(bitmap)
    @contents = bitmap
    RGSS::Native.window_set_contents(@native_id, @contents&.native_id)
  end

  def x=(value)
    @x = value.to_i
    RGSS::Native.window_set_x(@native_id, @x)
  end

  def y=(value)
    @y = value.to_i
    RGSS::Native.window_set_y(@native_id, @y)
  end

  def z=(value)
    @z = value.to_i
    RGSS::Native.window_set_z(@native_id, @z)
  end

  def width=(value)
    @width = value.to_i
    RGSS::Native.window_set_width(@native_id, @width)
  end

  def height=(value)
    @height = value.to_i
    RGSS::Native.window_set_height(@native_id, @height)
  end

  def ox=(value)
    @ox = value.to_i
    RGSS::Native.window_set_ox(@native_id, @ox)
  end

  def oy=(value)
    @oy = value.to_i
    RGSS::Native.window_set_oy(@native_id, @oy)
  end

  def opacity=(value)
    @opacity = value.to_i
    RGSS::Native.window_set_opacity(@native_id, @opacity)
  end

  def back_opacity=(value)
    @back_opacity = value.to_i
    RGSS::Native.window_set_back_opacity(@native_id, @back_opacity)
  end

  def contents_opacity=(value)
    @contents_opacity = value.to_i
    RGSS::Native.window_set_contents_opacity(@native_id, @contents_opacity)
  end

  def openness=(value)
    @openness = value.to_i.clamp(0, 255)
    RGSS::Native.window_set_openness(@native_id, @openness)
  end

  def visible=(value)
    @visible = !!value
    RGSS::Native.window_set_visible(@native_id, @visible)
  end

  def active=(value)
    @active = !!value
    RGSS::Native.window_set_active(@native_id, @active)
  end

  def pause=(value)
    @pause = !!value
    RGSS::Native.window_set_pause(@native_id, @pause)
  end

  def tone=(value)
    @tone = value.is_a?(Tone) ? value.dup : Tone.new
    RGSS::Native.window_set_tone(@native_id, @tone.red, @tone.green, @tone.blue, @tone.gray)
  end

  def color=(value)
    @color = value.is_a?(Color) ? value.dup : Color.new(0, 0, 0, 0)
    RGSS::Native.window_set_color(@native_id, @color.red, @color.green, @color.blue, @color.alpha)
  end

  def cursor_rect=(rect)
    @cursor_rect = rect.dup
    RGSS::Native.window_set_cursor_rect(@native_id, @cursor_rect.x, @cursor_rect.y, @cursor_rect.width, @cursor_rect.height)
  end

  def open
    self.openness = 255
  end

  def close
    self.openness = 0
  end

  def update; end

  private

  def sync_all
    RGSS::Native.window_set_viewport(@native_id, @viewport&.native_id)
    RGSS::Native.window_set_windowskin(@native_id, @windowskin&.native_id)
    RGSS::Native.window_set_contents(@native_id, @contents&.native_id)
    RGSS::Native.window_set_x(@native_id, @x)
    RGSS::Native.window_set_y(@native_id, @y)
    RGSS::Native.window_set_z(@native_id, @z)
    RGSS::Native.window_set_width(@native_id, @width)
    RGSS::Native.window_set_height(@native_id, @height)
    RGSS::Native.window_set_ox(@native_id, @ox)
    RGSS::Native.window_set_oy(@native_id, @oy)
    RGSS::Native.window_set_opacity(@native_id, @opacity)
    RGSS::Native.window_set_back_opacity(@native_id, @back_opacity)
    RGSS::Native.window_set_contents_opacity(@native_id, @contents_opacity)
    RGSS::Native.window_set_openness(@native_id, @openness)
    RGSS::Native.window_set_visible(@native_id, @visible)
    RGSS::Native.window_set_active(@native_id, @active)
    RGSS::Native.window_set_pause(@native_id, @pause)
    RGSS::Native.window_set_tone(@native_id, @tone.red, @tone.green, @tone.blue, @tone.gray)
    RGSS::Native.window_set_color(@native_id, @color.red, @color.green, @color.blue, @color.alpha)
    RGSS::Native.window_set_cursor_rect(@native_id, @cursor_rect.x, @cursor_rect.y, @cursor_rect.width, @cursor_rect.height)
  end
end

class Tilemap
  attr_reader :viewport, :tileset, :autotiles, :bitmaps, :map_data, :flash_data,
              :ox, :oy, :visible, :priorities, :opacity, :blend_type, :tone, :color, :native_id

  def initialize(viewport = nil)
    @viewport = viewport
    @tileset = nil
    @map_data = nil
    @flash_data = nil
    @priorities = nil
    @ox = 0
    @oy = 0
    @visible = true
    @opacity = 255
    @blend_type = 0
    @tone = Tone.new
    @color = Color.new(0, 0, 0, 0)
    @disposed = false
    @native_id = RGSS::Native.tilemap_create(@viewport&.native_id)
    @autotiles = AutotileProxy.new(self, 7)
    @bitmaps = @autotiles
    sync_all
  end

  def disposed?
    @disposed
  end

  def dispose
    return if disposed?
    RGSS::Native.tilemap_dispose(@native_id)
    @disposed = true
  end

  def viewport=(viewport)
    @viewport = viewport
    RGSS::Native.tilemap_set_viewport(@native_id, @viewport&.native_id)
  end

  def tileset=(bitmap)
    @tileset = bitmap
    RGSS::Native.tilemap_set_tileset(@native_id, @tileset&.native_id)
  end

  def autotiles=(array)
    @autotiles.replace(array)
  end

  def bitmaps=(array)
    self.autotiles = array
  end

  def map_data=(table)
    @map_data = table
    sync_map_data
  end

  def priorities=(table)
    @priorities = table
    sync_priorities
  end

  def flash_data=(table)
    @flash_data = table
    sync_flash_data
  end

  def ox=(value)
    @ox = value.to_i
    RGSS::Native.tilemap_set_ox(@native_id, @ox)
  end

  def oy=(value)
    @oy = value.to_i
    RGSS::Native.tilemap_set_oy(@native_id, @oy)
  end

  def visible=(value)
    @visible = !!value
    RGSS::Native.tilemap_set_visible(@native_id, @visible)
  end

  def opacity=(value)
    @opacity = value.to_i.clamp(0, 255)
    RGSS::Native.tilemap_set_opacity(@native_id, @opacity)
  end

  def blend_type=(value)
    @blend_type = value.to_i
    RGSS::Native.tilemap_set_blend_type(@native_id, @blend_type)
  end

  def tone=(value)
    @tone = value.is_a?(Tone) ? value.dup : Tone.new
    RGSS::Native.tilemap_set_tone(@native_id, @tone.red, @tone.green, @tone.blue, @tone.gray)
  end

  def color=(value)
    @color = value.is_a?(Color) ? value.dup : Color.new(0, 0, 0, 0)
    RGSS::Native.tilemap_set_color(@native_id, @color.red, @color.green, @color.blue, @color.alpha)
  end

  def update
    RGSS::Native.tilemap_update(@native_id)
  end

  private

  def sync_all
    RGSS::Native.tilemap_set_viewport(@native_id, @viewport&.native_id)
    RGSS::Native.tilemap_set_tileset(@native_id, @tileset&.native_id)
    @autotiles.each_with_index do |bitmap, index|
      apply_autotile(index, bitmap)
    end
    sync_map_data
    sync_priorities
    RGSS::Native.tilemap_set_ox(@native_id, @ox)
    RGSS::Native.tilemap_set_oy(@native_id, @oy)
    RGSS::Native.tilemap_set_visible(@native_id, @visible)
    RGSS::Native.tilemap_set_opacity(@native_id, @opacity)
    RGSS::Native.tilemap_set_blend_type(@native_id, @blend_type)
    RGSS::Native.tilemap_set_tone(@native_id, @tone.red, @tone.green, @tone.blue, @tone.gray)
    RGSS::Native.tilemap_set_color(@native_id, @color.red, @color.green, @color.blue, @color.alpha)
    sync_flash_data
  end

  def sync_map_data
    return unless @map_data
    RGSS::Native.tilemap_set_map_data(
      @native_id,
      @map_data.xsize,
      @map_data.ysize,
      @map_data.zsize,
      @map_data.to_native_s16
    )
  end

  def sync_priorities
    return unless @priorities
    RGSS::Native.tilemap_set_priorities(
      @native_id,
      @priorities.xsize,
      @priorities.to_native_s16
    )
  end

  def sync_flash_data
    return unless @flash_data
    RGSS::Native.tilemap_set_flash_data(
      @native_id,
      @flash_data.xsize,
      @flash_data.ysize,
      @flash_data.to_native_s16
    )
  end

  def apply_autotile(index, bitmap)
    RGSS::Native.tilemap_set_autotile(@native_id, index, bitmap&.native_id)
  end

  class AutotileProxy
    include Enumerable

    def initialize(owner, size)
      @owner = owner
      @data = Array.new(size)
    end

    def [](index)
      @data[index]
    end

    def []=(index, value)
      return unless index.between?(0, @data.length - 1)
      @data[index] = value
      @owner.send(:apply_autotile, index, value)
    end

    def each(&block)
      @data.each(&block)
    end

    def replace(values)
      values = Array(values)
      @data.length.times do |index|
        self[index] = values[index]
      end
    end
  end
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

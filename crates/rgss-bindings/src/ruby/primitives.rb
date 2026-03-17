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

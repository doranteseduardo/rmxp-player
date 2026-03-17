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

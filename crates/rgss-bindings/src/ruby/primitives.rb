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

module RGSS
  module Compat
    INSTANCE_CLASS_METHOD = ::Object.instance_method(:class)

    def self.basic_class(obj)
      INSTANCE_CLASS_METHOD.bind(obj).call
    end

    def self.inject_class_method(mod)
      mod.class_eval do
        define_method(:class) { RGSS::Compat.basic_class(self) }
      end
    end

    def self.inject_singleton_class_method(mod)
      mod.singleton_class.class_eval do
        define_method(:class) { RGSS::Compat.basic_class(self) }
      end
    end

    def self.report_class_change(target, action)
      script = ($RGSS_CURRENT_SCRIPT || '(unknown script)').to_s
      location = caller_locations(2, 10)&.find { |loc| loc.path && !loc.path.include?(__FILE__) }
      site = location ? "#{location.path}:#{location.lineno}" : '(no Ruby caller)'
      warn("[RGSS] #{script} #{action} #{target}#class @ #{site}")
    end

    module ModulePatch
      def undef_method(*symbols)
        restore = symbols.any? { |sym| sym.to_sym == :class }
        super
        if restore
          RGSS::Compat.report_class_change(self, :undef_method)
          RGSS::Compat.inject_class_method(self)
        end
      end

      def remove_method(*symbols)
        restore = symbols.any? { |sym| sym.to_sym == :class }
        super
        if restore
          RGSS::Compat.report_class_change(self, :remove_method)
          RGSS::Compat.inject_class_method(self)
        end
      end
    end

    module KernelSingletonPatch
      def undef_method(*symbols)
        restore = symbols.any? { |sym| sym.to_sym == :class }
        super
        if restore
          RGSS::Compat.report_class_change(self, :undef_singleton_method)
          RGSS::Compat.inject_singleton_class_method(Kernel)
        end
      end

      def remove_method(*symbols)
        restore = symbols.any? { |sym| sym.to_sym == :class }
        super
        if restore
          RGSS::Compat.report_class_change(self, :remove_singleton_method)
          RGSS::Compat.inject_singleton_class_method(Kernel)
        end
      end
    end
  end
end

RGSS::Compat.inject_class_method(Object)
RGSS::Compat.inject_singleton_class_method(Kernel)

Module.prepend(RGSS::Compat::ModulePatch)
Kernel.singleton_class.prepend(RGSS::Compat::KernelSingletonPatch)

unless String.method_defined?(:clone)
  class String
    def clone(*)
      dup
    end
  end
end

module Kernel
  def data_path(relative = '')
    RGSS::Native.project_path(relative.to_s)
  end

  def load_data(filename)
    path = resolve_rgss_read_path(filename)
    File.open(path, 'rb') { |f| Marshal.send(:load, f) }
  end

  def save_data(object, filename)
    path = resolve_rgss_write_path(filename)
    File.open(path, 'wb') { |f| Marshal.send(:dump, object, f) }
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

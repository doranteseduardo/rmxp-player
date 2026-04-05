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
    @pending_suspend = false
    @low_memory = false

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

      # Called by Rust to wrap the Main script source in a Fiber driven by the
      # event loop.  Both PE-style (mainFunction directly) and standard-style
      # (rgss_main { block }) games benefit: Graphics.update -> Fiber.yield
      # yields control to the event loop for rendering without ever blocking it.
      def install_main_from_source(source, label)
        @main_fiber = Fiber.new do
          begin
            # rubocop:disable Security/Eval
            TOPLEVEL_BINDING.eval(source, "(\#{label})", 1)
            # rubocop:enable Security/Eval
          rescue ::SystemExit
            # clean exit
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

      def notify_suspend
        @pending_suspend = true
      end

      def notify_resume
        @pending_suspend = false
      end

      def notify_low_memory
        @low_memory = true
      end

      def pending_events?
        !!@pending_suspend
      end

      def consume_low_memory!
        flag = @low_memory
        @low_memory = false
        flag
      end
    end
  end
end

class Hangup < Exception; end

# Raised by the game (or the engine on F12) to restart the entire script loop.
class Reset < Exception; end

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
        restore_class = symbols.any? { |sym| sym.to_sym == :class }
        restore_clone = symbols.any? { |sym| sym.to_sym == :clone }
        super
        if restore_class
          RGSS::Compat.report_class_change(self, :undef_method)
          RGSS::Compat.inject_class_method(self)
        end
        if restore_clone
          mod = self
          mod.class_eval { define_method(:clone) { |*| dup } } rescue nil
        end
      end

      def remove_method(*symbols)
        restore_class = symbols.any? { |sym| sym.to_sym == :class }
        restore_clone = symbols.any? { |sym| sym.to_sym == :clone }
        super
        if restore_class
          RGSS::Compat.report_class_change(self, :remove_method)
          RGSS::Compat.inject_class_method(self)
        end
        if restore_clone
          mod = self
          mod.class_eval { define_method(:clone) { |*| dup } } rescue nil
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

# Capture Marshal.dump before any game script can shadow it.
# For Marshal.load we use RGSS::Native.marshal_load which calls rb_marshal_load
# at the C level, bypassing any Ruby-level method dispatch corruption.
RGSS_MARSHAL_DUMP = Marshal.method(:dump)

module Kernel
  def data_path(relative = '')
    RGSS::Native.project_path(relative.to_s)
  end

  def load_data(filename)
    path = resolve_rgss_read_path(filename)
    RGSS::Native.marshal_load(path)
  end

  def save_data(object, filename)
    path = resolve_rgss_write_path(filename)
    File.open(path, 'wb') { |f| RGSS_MARSHAL_DUMP.call(object, f) }
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

end

# Numeric conversion safeguards.
# Ruby 3.2 defines Float#to_f at the C level. Some PE protection mechanisms
# have been observed to clear the Float method table, dropping inherited C
# methods. Define a Ruby-level fallback on Numeric so it is always reachable
# through the superclass chain even if Float's own entry disappears.
class Numeric
  def to_f; self + 0.0; end unless method_defined?(:to_f)
end
class Float
  def to_f; self; end
  def to_i; (self < 0 ? -(-self).floor : floor); end unless method_defined?(:to_i)
  def to_r; Rational(self); end unless method_defined?(:to_r)
end

# mkxp-z GIF animation extension stubs for Bitmap.
# We don't support animated GIFs — all bitmaps are static. Scripts that guard
# on animated? will skip GIF-specific paths; the rest are safe no-ops.
class Bitmap
  def animated?;             false; end
  def play;                  nil;   end
  def stop;                  nil;   end
  def goto_and_stop(_frame); nil;   end
  def current_frame;         0;     end
  def frame_count;           1;     end
  def frame_rate;            nil;   end
  # mkxp-z "mega texture" check (very large tilesets split into columns).
  def mega?;                 false; end
end

module Graphics
  class << self
    alias_method :__update_native, :update

    # One-shot hook: on the first Graphics.update call the game loop is
    # running and all PE protection scripts have already had a chance to
    # tamper with standard methods.  Restore any that were replaced with
    # raising stubs (PE does this instead of undef_method, so our
    # ModulePatch cannot intercept it).
    @_rgss_methods_restored = false

    def update
      unless @_rgss_methods_restored
        @_rgss_methods_restored = true
        _restore_stripped_methods
      end
      __update_native
    end

    private

    def _restore_stripped_methods
      return unless defined?(RPG)
      RPG.constants.each do |const|
        klass = RPG.const_get(const) rescue next
        next unless klass.is_a?(Module)
        # Unconditionally redefine :clone — PE may have replaced it with a
        # stub that raises NoMethodError, which method_defined? reports as true.
        klass.class_eval { define_method(:clone) { |*| dup } } rescue nil
      end
    end

    public

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

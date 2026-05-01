# Compatibility shims for Ruby APIs that existed in RPG Maker's bundled Ruby
# but were renamed or changed in modern Ruby (3.x). Ported from mkxp-z's
# ruby_classic_wrap.rb (CC0, WaywardHeart 2023).

class Hash
  alias_method :index, :key unless method_defined?(:index)
end

class Object
  TRUE  = true  unless const_defined?("TRUE")
  FALSE = false unless const_defined?("FALSE")
  NIL   = nil   unless const_defined?("NIL")

  alias_method :id,   :object_id unless method_defined?(:id)
  alias_method :type, :class     unless method_defined?(:type)
end

class NilClass
  def id
    4
  end

  def to_i;   0;   end unless method_defined?(:to_i)
  def to_f;   0.0; end unless method_defined?(:to_f)
  def to_s;   "";  end unless method_defined?(:to_s)
  def to_a;   [];  end unless method_defined?(:to_a)
  def to_r;   0r;  end unless method_defined?(:to_r)
end

class TrueClass
  def id
    2
  end
end

# BasicObject#initialize shim intentionally omitted: redefining BasicObject
# methods inside an embedded MRI 3.2 VM triggers rb_estimate_iv_count on the
# root class before its shape data is fully set up, causing a segfault.
# Ruby 3.x already forwards any arguments through BasicObject.new, so the
# shim is not needed.

# Seed global RNG — required in embedded Ruby before Array#sample / rand work.
begin
  srand(Time.now.to_i)
rescue
  srand(Process.pid)
end

# Defensive encoding normalisation inside Interpreter#command_101 and
# command_102. Despite the marshal_load post-processor, dialog parameters
# sometimes still arrive ASCII-8BIT (especially from event lists copied via
# `Marshal.load(Marshal.dump(@list))` in setup_choices). When the string is
# ASCII-8BIT, PE's downstream `text.scan(/./m)` byte-iterates and splits
# multibyte glyphs (Pokémon → PokÃ©mon). Force the encoding here as a last
# resort before PE processes the text.
module InterpreterTextEncoding
  def command_101
    fix_param_encoding!(0)
    super
  end

  def command_102
    fix_param_encoding!(0)
    super
  end

  private

  def fix_param_encoding!(idx)
    s = (@list && @index && @list[@index] && @list[@index].parameters[idx]) rescue nil
    return unless s.is_a?(::String)
    return unless s.encoding == ::Encoding::ASCII_8BIT
    utf8 = s.dup.force_encoding(::Encoding::UTF_8)
    if utf8.valid_encoding?
      s.force_encoding(::Encoding::UTF_8) rescue nil
    else
      converted = s.dup
        .force_encoding(::Encoding::WINDOWS_1252)
        .encode(::Encoding::UTF_8, invalid: :replace, undef: :replace, replace: '?') rescue nil
      s.replace(converted) if converted
    end
  end
end

TracePoint.new(:end) do |tp|
  s = tp.self
  next unless s.is_a?(Module) && s.name == 'Interpreter'
  next unless s.method_defined?(:command_101)
  next if s.include?(InterpreterTextEncoding)
  s.prepend(InterpreterTextEncoding)
end.enable

# Defensive last-mile: force-encode any String reaching the global
# text-display helpers. Catches PE scripts that call pbMessage / pbDisplay
# directly (without going through Interpreter#command_101). Top-level
# methods are private instance methods of Object.
module RGSSTextEncodingShim
  TARGETS = %i[pbMessage pbMessageDisplay pbDisplayMessage pbConfirmMessage
               pbConfirmMessageSerious pbMessageChooseNumber].freeze

  def self.fix!(value)
    return value unless value.is_a?(::String)
    return value unless value.encoding == ::Encoding::ASCII_8BIT
    utf8 = value.dup.force_encoding(::Encoding::UTF_8)
    if utf8.valid_encoding?
      value.force_encoding(::Encoding::UTF_8) rescue nil
      return value
    end
    converted = value.dup
      .force_encoding(::Encoding::WINDOWS_1252)
      .encode(::Encoding::UTF_8, invalid: :replace, undef: :replace, replace: '?') rescue nil
    value.replace(converted) if converted
    value
  end

  TARGETS.each do |name|
    target = name
    define_method(target) do |*args, &block|
      args[0] = RGSSTextEncodingShim.fix!(args[0]) if !args.empty?
      method("_rgss_orig_#{target}").call(*args, &block)
    end
  end
end

# Watch for top-level method definitions. Hook Object.method_added so we
# can rewrap target methods *immediately* after PE defines them.
class << Object
  alias_method :_rgss_orig_method_added, :method_added rescue nil
  def method_added(name)
    super if defined?(super)
    return unless RGSSTextEncodingShim::TARGETS.include?(name)
    return if name.to_s.start_with?('_rgss_orig_')
    return if private_method_defined?(:"_rgss_orig_#{name}") ||
              method_defined?(:"_rgss_orig_#{name}")
    alias_method :"_rgss_orig_#{name}", name
    define_method(name, RGSSTextEncodingShim.instance_method(name))
    private name
  end
end


# RGSS-era convention: PE calls `Kernel.pbX(...)` to invoke top-level helper
# methods regardless of context (`Kernel.pbShowCommands`, `Kernel.pbMessage`,
# etc.). In modern Ruby, top-level `def pbX` defines a private instance
# method of Object — `Kernel.pbX(...)` raises NoMethodError because pbX is
# not a singleton method of Kernel. Forward unknown Kernel class methods to
# TOPLEVEL_BINDING's main object, which has the top-level methods accessible
# via send (private OK).
class << Kernel
  alias_method :_rgss_orig_method_missing, :method_missing
  def method_missing(name, *args, &block)
    main = TOPLEVEL_BINDING.receiver
    return main.send(name, *args, &block) if main.respond_to?(name, true)
    _rgss_orig_method_missing(name, *args, &block)
  end
  def respond_to_missing?(name, include_private = false)
    main = TOPLEVEL_BINDING.receiver
    main.respond_to?(name, true) || super
  end
end

# Self-heal a stuck @message_waiting=true on the map interpreter. We've
# observed (Oak intro path in PE 21.1) that on the first observed
# Interpreter#update the flag is already true even though no message window
# exists and pbMessage was never reached. Without recovery, the interpreter
# returns at line 104 of update on every frame and the autorun never
# advances past the first Show Text.
#
# Recovery rule: clear @message_waiting only when no message window is
# actually showing (pbCreateMessageWindow sets $game_temp.message_window_showing
# and pbDisposeMessageWindow clears it). With a real message window up, leave
# the flag alone so legit pbMessage waits keep working.
#
# Root cause is open — see SESSION_PROGRESS.md. This is a band-aid.
module InterpreterMessageWaitingRecovery
  def update
    if @message_waiting
      gt = (defined?($game_temp) ? $game_temp : nil)
      showing = gt && gt.respond_to?(:message_window_showing) && gt.message_window_showing
      @message_waiting = false unless showing
    end
    super
  end
end

TracePoint.new(:end) do |tp|
  begin
    s = tp.self
    next unless s.is_a?(Module) && s.name == 'Interpreter'
    next unless s.method_defined?(:update) && s.method_defined?(:command_101)
    next if s.include?(InterpreterMessageWaitingRecovery)
    s.prepend(InterpreterMessageWaitingRecovery)
  rescue
    # never break script load
  end
end.enable


class Array
  unless method_defined?(:sample)
    def sample(n = nil)
      return (n.nil? ? nil : []) if empty?
      if n.nil?
        self[rand(size)]
      else
        n = [n.to_int, size].min
        result = dup
        n.times.map { result.delete_at(rand(result.size)) }
      end
    end
  end

  unless method_defined?(:shuffle)
    def shuffle
      result = dup
      (result.size - 1).downto(1) do |i|
        j = rand(i + 1)
        result[i], result[j] = result[j], result[i]
      end
      result
    end
  end
end

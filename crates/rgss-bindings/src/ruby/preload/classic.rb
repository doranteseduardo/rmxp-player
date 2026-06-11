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

# Recover a stranded @message_waiting flag on the map interpreter.
#
# Symptom (Oak intro path in PE 21.1): on the first Interpreter#update the flag
# is already true with no message window up, so update returns early at
# 033_Interpreter.rb:104 every frame and the autorun never advances past the
# first Show Text.
#
# Mechanism: command_101/102/103 set @message_waiting=true, call the BLOCKING
# pbMessage/pbShowCommands, then set it false. That clear is skipped only on a
# non-local exit — an exception unwinding past the reset, or the main Fiber being
# torn down/reinstalled mid-message while $game_system.map_interpreter (a global)
# survives. The flag is then orphaned true.
#
# Fix below is in two precise, vanilla-safe parts (see the module): an `ensure`
# on the message commands for the exception path, and a scoped update fallback
# for the Fiber-teardown path. Both no-op for games that don't use PE's message
# system, so vanilla RMXP (which manages the flag via Window_Message) is untouched.
module InterpreterMessageWaitingRecovery
  # Returns the PE message-system state:
  #   true  -> a message window is currently showing
  #   false -> PE message system present, no window showing
  #   nil   -> game does not use PE's message system (e.g. vanilla RMXP)
  # Vanilla RMXP has no $game_temp.message_window_showing and drives
  # @message_waiting through Window_Message, so we must NOT touch the flag there.
  def self.message_window_state
    gt = (defined?($game_temp) ? $game_temp : nil)
    return nil unless gt && gt.respond_to?(:message_window_showing)
    gt.message_window_showing ? true : false
  end

  # Precise root-cause guard: command_101/102/103 set @message_waiting=true, call
  # the blocking pbMessage/pbShowCommands, then clear it. If that body exits
  # non-locally (an exception unwinds past the explicit reset), the flag is left
  # stranded. This `ensure` clears ONLY a flag this command itself raised, and
  # only when no message window is up — so normal completion (flag already false
  # here) and legitimate waits are untouched. No per-frame polling, no race.
  [:command_101, :command_102, :command_103].each do |cmd|
    define_method(cmd) do |*args, &blk|
      raised = !@message_waiting
      begin
        super(*args, &blk)
      ensure
        if raised && @message_waiting &&
           InterpreterMessageWaitingRecovery.message_window_state == false
          @message_waiting = false
        end
      end
    end
  end

  # Fallback for the case the `ensure` above can't catch: the main Fiber is torn
  # down and reinstalled mid-message while $game_system.map_interpreter survives,
  # so the stack that would have cleared the flag is discarded. On the next
  # update the flag is stuck true with no window. Scoped to the PE message system
  # (state == false) so vanilla games are never affected.
  def update
    if @message_waiting && InterpreterMessageWaitingRecovery.message_window_state == false
      @message_waiting = false
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

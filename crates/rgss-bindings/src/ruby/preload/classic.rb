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

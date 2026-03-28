# Compatibility aliases for games that call the older mkxp (Ancurio) API names
# instead of the mkxp-z names. Ported from mkxp-z's mkxp_wrap.rb (CC0).

module MKXP
  class << self
    def data_directory(*args)
      System.data_directory(*args)
    end

    def puts(*args)
      System.puts(*args)
    end

    def raw_key_states(*args)
      states = Input.raw_key_states(*args)
      class << states
        def getbyte(byte)
          self[byte] ? 1 : 0
        end

        def setbyte(byte, val)
          self[byte] = val == 0 ? false : true
        end
      end
      states
    end

    def mouse_in_window(*args)
      Input.mouse_in_window(*args)
    end
  end
end

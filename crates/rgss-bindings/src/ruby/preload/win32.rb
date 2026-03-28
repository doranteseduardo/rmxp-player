# Cross-platform Win32API shim. Routes common Win32 calls to native equivalents
# so games that call Win32API don't crash on non-Windows platforms.
# Ported from mkxp-z's win32_wrap.rb (CC0, Ancurio 2014, Splendide Imaginarius 2023-2024).

module Scancodes
  SDL = {
    :UNKNOWN => 0x00,
    :A => 0x04, :B => 0x05, :C => 0x06, :D => 0x07,
    :E => 0x08, :F => 0x09, :G => 0x0A, :H => 0x0B,
    :I => 0x0C, :J => 0x0D, :K => 0x0E, :L => 0x0F,
    :M => 0x10, :N => 0x11, :O => 0x12, :P => 0x13,
    :Q => 0x14, :R => 0x15, :S => 0x16, :T => 0x17,
    :U => 0x18, :V => 0x19, :W => 0x1A, :X => 0x1B,
    :Y => 0x1C, :Z => 0x1D, :N1 => 0x1E, :N2 => 0x1F,
    :N3 => 0x20, :N4 => 0x21, :N5 => 0x22, :N6 => 0x23,
    :N7 => 0x24, :N8 => 0x25, :N9 => 0x26, :N0 => 0x27,
    :RETURN => 0x28, :ESCAPE => 0x29, :BACKSPACE => 0x2A, :TAB => 0x2B,
    :SPACE => 0x2C, :MINUS => 0x2D, :EQUALS => 0x2E, :LEFTBRACKET => 0x2F,
    :RIGHTBRACKET => 0x30, :BACKSLASH => 0x31, :NONUSHASH => 0x32, :SEMICOLON => 0x33,
    :APOSTROPHE => 0x34, :GRAVE => 0x35, :COMMA => 0x36, :PERIOD => 0x37,
    :SLASH => 0x38, :CAPSLOCK => 0x39, :F1 => 0x3A, :F2 => 0x3B,
    :F3 => 0x3C, :F4 => 0x3D, :F5 => 0x3E, :F6 => 0x3F,
    :F7 => 0x40, :F8 => 0x41, :F9 => 0x42, :F10 => 0x43,
    :F11 => 0x44, :F12 => 0x45, :PRINTSCREEN => 0x46, :SCROLLLOCK => 0x47,
    :PAUSE => 0x48, :INSERT => 0x49, :HOME => 0x4A, :PAGEUP => 0x4B,
    :DELETE => 0x4C, :END => 0x4D, :PAGEDOWN => 0x4E, :RIGHT => 0x4F,
    :LEFT => 0x50, :DOWN => 0x51, :UP => 0x52, :NUMLOCKCLEAR => 0x53,
    :KP_DIVIDE => 0x54, :KP_MULTIPLY => 0x55, :KP_MINUS => 0x56, :KP_PLUS => 0x57,
    :KP_ENTER => 0x58, :KP_1 => 0x59, :KP_2 => 0x5A, :KP_3 => 0x5B,
    :KP_4 => 0x5C, :KP_5 => 0x5D, :KP_6 => 0x5E, :KP_7 => 0x5F,
    :KP_8 => 0x60, :KP_9 => 0x61, :KP_0 => 0x62, :KP_PERIOD => 0x63,
    :NONUSBACKSLASH => 0x64, :APPLICATION => 0x65, :POWER => 0x66, :KP_EQUALS => 0x67,
    :F13 => 0x68, :F14 => 0x69, :F15 => 0x6A, :F16 => 0x6B,
    :F17 => 0x6C, :F18 => 0x6D, :F19 => 0x6E, :F20 => 0x6F,
    :F21 => 0x70, :F22 => 0x71, :F23 => 0x72, :F24 => 0x73,
    :EXECUTE => 0x74, :HELP => 0x75, :MENU => 0x76, :SELECT => 0x77,
    :STOP => 0x78, :AGAIN => 0x79, :UNDO => 0x7A, :CUT => 0x7B,
    :COPY => 0x7C, :PASTE => 0x7D, :FIND => 0x7E, :MUTE => 0x7F,
    :VOLUMEUP => 0x80, :VOLUMEDOWN => 0x81,
    :KP_COMMA => 0x85,
    :LCTRL => 0xE0, :LSHIFT => 0xE1, :LALT => 0xE2, :LGUI => 0xE3,
    :RCTRL => 0xE4, :RSHIFT => 0xE5, :RALT => 0xE6, :RGUI => 0xE7,
  }

  SDL.default = SDL[:UNKNOWN]

  WIN32 = {
    :LBUTTON => 0x01, :RBUTTON => 0x02, :MBUTTON => 0x04,
    :BACK => 0x08, :TAB => 0x09, :RETURN => 0x0D, :SHIFT => 0x10,
    :CONTROL => 0x11, :MENU => 0x12, :PAUSE => 0x13, :CAPITAL => 0x14,
    :ESCAPE => 0x1B, :SPACE => 0x20, :PRIOR => 0x21, :NEXT => 0x22,
    :END => 0x23, :HOME => 0x24, :LEFT => 0x25, :UP => 0x26,
    :RIGHT => 0x27, :DOWN => 0x28, :PRINT => 0x2A, :INSERT => 0x2D,
    :DELETE => 0x2E,
    :N0 => 0x30, :N1 => 0x31, :N2 => 0x32, :N3 => 0x33,
    :N4 => 0x34, :N5 => 0x35, :N6 => 0x36, :N7 => 0x37,
    :N8 => 0x38, :N9 => 0x39,
    :A => 0x41, :B => 0x42, :C => 0x43, :D => 0x44, :E => 0x45, :F => 0x46,
    :G => 0x47, :H => 0x48, :I => 0x49, :J => 0x4A, :K => 0x4B, :L => 0x4C,
    :M => 0x4D, :N => 0x4E, :O => 0x4F, :P => 0x50, :Q => 0x51, :R => 0x52,
    :S => 0x53, :T => 0x54, :U => 0x55, :V => 0x56, :W => 0x57, :X => 0x58,
    :Y => 0x59, :Z => 0x5A,
    :LWIN => 0x5B, :RWIN => 0x5C,
    :NUMPAD0 => 0x60, :NUMPAD1 => 0x61, :NUMPAD2 => 0x62, :NUMPAD3 => 0x63,
    :NUMPAD4 => 0x64, :NUMPAD5 => 0x65, :NUMPAD6 => 0x66, :NUMPAD7 => 0x67,
    :NUMPAD8 => 0x68, :NUMPAD9 => 0x69,
    :MULTIPLY => 0x6A, :ADD => 0x6B, :SEPARATOR => 0x6C, :SUBSTRACT => 0x6D,
    :DECIMAL => 0x6E, :DIVIDE => 0x6F,
    :F1 => 0x70, :F2 => 0x71, :F3 => 0x72, :F4 => 0x73,
    :F5 => 0x74, :F6 => 0x75, :F7 => 0x76, :F8 => 0x77,
    :F9 => 0x78, :F10 => 0x79, :F11 => 0x7A, :F12 => 0x7B,
    :F13 => 0x7C, :F14 => 0x7D, :F15 => 0x7E, :F16 => 0x7F,
    :F17 => 0x80, :F18 => 0x81, :F19 => 0x82, :F20 => 0x83,
    :F21 => 0x84, :F22 => 0x85, :F23 => 0x86, :F24 => 0x87,
    :NUMLOCK => 0x90, :SCROLL => 0x91,
    :LSHIFT => 0xA0, :RSHIFT => 0xA1, :LCONTROL => 0xA2, :RCONTROL => 0xA3,
    :LMENU => 0xA4, :RMENU => 0xA5,
    :OEM_1 => 0xBA, :OEM_PLUS => 0xBB, :OEM_COMMA => 0xBC,
    :OEM_MINUS => 0xBD, :OEM_PERIOD => 0xBE, :OEM_2 => 0xBF,
    :OEM_3 => 0xC0, :OEM_4 => 0xDB, :OEM_5 => 0xDC,
    :OEM_6 => 0xDD, :OEM_7 => 0xDE,
  }

  WIN32INV = WIN32.invert

  WIN2SDL = {
    :BACK => :BACKSPACE, :CAPITAL => :CAPSLOCK,
    :PRIOR => :PAGEUP, :NEXT => :PAGEDOWN,
    :PRINT => :PRINTSCREEN,
    :LWIN => :LGUI, :RWIN => :RGUI,
    :NUMPAD0 => :KP_0, :NUMPAD1 => :KP_1, :NUMPAD2 => :KP_2, :NUMPAD3 => :KP_3,
    :NUMPAD4 => :KP_4, :NUMPAD5 => :KP_5, :NUMPAD6 => :KP_6, :NUMPAD7 => :KP_7,
    :NUMPAD8 => :KP_8, :NUMPAD9 => :KP_9,
    :MULTIPLY => :KP_MULTIPLY, :ADD => :KP_PLUS, :SUBSTRACT => :KP_MINUS,
    :DECIMAL => :KP_DECIMAL, :DIVIDE => :KP_DIVIDE,
    :NUMLOCK => :NUMLOCKCLEAR, :SCROLL => :SCROLLLOCK,
    :LCONTROL => :LCTRL, :RCONTROL => :RCTRL,
    :LMENU => :LALT, :RMENU => :RALT,
    :OEM_1 => :SEMICOLON, :OEM_PLUS => :EQUALS, :OEM_COMMA => :COMMA,
    :OEM_MINUS => :MINUS, :OEM_PERIOD => :PERIOD, :OEM_2 => :SLASH,
    :OEM_3 => :GRAVE, :OEM_4 => :LEFTBRACKET, :OEM_5 => :BACKSLASH,
    :OEM_6 => :RIGHTBRACKET, :OEM_7 => :APOSTROPHE,
  }

  WIN2SDL.default = :UNKNOWN
end

$win32KeyStates = nil

module Graphics
  class << self
    alias_method :_win32wrap_update, :update unless method_defined?(:_win32wrap_update)

    def update
      _win32wrap_update
      $win32KeyStates = nil
    end
  end
end

def get_raw_keystates
  $win32KeyStates ||= Input.raw_key_states
end

def common_keystate(vkey)
  vkey_name = Scancodes::WIN32INV[vkey]
  states    = get_raw_keystates
  pressed   = false

  case vkey_name
  when :LBUTTON  then pressed = Input.press?(Input::MOUSELEFT)
  when :RBUTTON  then pressed = Input.press?(Input::MOUSERIGHT)
  when :MBUTTON  then pressed = Input.press?(Input::MOUSEMIDDLE)
  when :SHIFT    then pressed = double_state(states, :LSHIFT, :RSHIFT)
  when :MENU     then pressed = double_state(states, :LALT, :RALT)
  when :CONTROL  then pressed = double_state(states, :LCTRL, :RCTRL)
  else
    scan = Scancodes::SDL.key?(vkey_name) ? vkey_name : Scancodes::WIN2SDL[vkey_name]
    pressed = state_pressed(states, scan)
  end

  pressed ? 1 : 0
end

def memcpy_string(dst, src)
  i = 0
  src.each_byte { |b| dst.setbyte(i, b); i += 1 }
end

def state_pressed(states, sdl_scan)
  states[Scancodes::SDL[sdl_scan]]
end

def double_state(states, left, right)
  state_pressed(states, left) || state_pressed(states, right)
end

module Win32API_Impl
  module User32
    class Keybd_event
      SEQ  = [[0xA4,0,0,0],[0xD,0,0,0],[0xD,0,2,0],[0xA4,0,2,0]]
      SEQ2 = [[0x12,0,0,0],[0xD,0,0,0],[0xD,0,2,0],[0x12,0,2,0]]

      def initialize
        @index = 0
      end

      def call(args)
        seq = [args[0], args[1], args[2], args[3]]
        if seq == SEQ[@index] || seq == SEQ2[@index]
          @index += 1
        else
          @index = 0
        end
        if @index == 4
          @index = 0
          Graphics.fullscreen = !Graphics.fullscreen
        end
      end
    end

    class GetKeyState
      def call(vkey)
        common_keystate(vkey[0])
      end
    end

    class GetAsyncKeyState
      PRESSED_BIT = (1 << 15)

      def call(vkey)
        common_keystate(vkey[0]) == 1 ? PRESSED_BIT : 0
      end
    end

    class GetKeyboardState
      PRESSED_BIT = 0x80

      def call(args)
        out_states = args[0]
        Scancodes::WIN32.each do |name, val|
          pressed = common_keystate(val) == 1
          out_states.setbyte(val, pressed ? PRESSED_BIT : 0)
        end
        1
      end
    end

    class ShowCursor
      def initialize
        @cursor_count = 0
      end

      def call(args)
        if args[0] == 1
          @cursor_count += 1
        else
          @cursor_count -= 1
        end
        Graphics.show_cursor = @cursor_count >= 0
      end
    end

    class GetCursorPos
      def call(args)
        out = [Input.mouse_x, Input.mouse_y].pack('ll')
        memcpy_string(args[0], out)
        1
      end
    end

    class GetClientRect
      def call(args)
        return 0 if args[0] != 42
        w = Graphics.width  rescue 640
        h = Graphics.height rescue 480
        memcpy_string(args[1], [0, 0, w, h].pack('l4'))
        1
      end
    end

    class ScreenToClient
      def call(_args)
        1
      end
    end

    class FindWindowA
      def call(args)
        args[0] == "RGSS Player" ? 42 : 0
      end
    end
  end
end

def kappatalize(s)
  s[0] = s[0].upcase
  s
end

class Win32API
  NATIVE_ON_WINDOWS = true  unless const_defined?("NATIVE_ON_WINDOWS")
  TOLERATE_ERRORS   = true  unless const_defined?("TOLERATE_ERRORS")
  LOG_NATIVE        = false unless const_defined?("LOG_NATIVE")

  def initialize(dll, func, *_args)
    @dll    = dll
    @func   = func
    @called = false

    dll_key  = kappatalize(dll.chomp(".dll"))
    func_key = kappatalize(func)

    on_windows = defined?(System) && System.respond_to?(:is_windows?) && System.is_windows?

    unless on_windows && NATIVE_ON_WINDOWS
      if Win32API_Impl.const_defined?(dll_key)
        dll_impl = Win32API_Impl.const_get(dll_key)
        if dll_impl.const_defined?(func_key)
          @impl = dll_impl.const_get(func_key).new
          return
        end
      end
    end

    # No implementation found — tolerate silently or raise.
  end

  def call(*args)
    return @impl.call(args) if @impl

    if TOLERATE_ERRORS
      unless @called
        warn("[Win32API] unimplemented #{@dll}:#{@func}") if defined?(System)
        @called = true
      end
      0
    else
      raise RuntimeError, "[Win32API] unimplemented #{@dll}:#{@func} #{args}"
    end
  end
end

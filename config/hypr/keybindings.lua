--------------------
----  APPS      ----
--------------------

local terminal = "ghostty"
local fileManager = "thunar"
local browser = "google-chrome"
local music = "spotify"
local passwordManager = "1password"
local messenger = "signal-desktop"
local webapp = "google-chrome --new-window --ozone-platform=wayland --app="

hl.bind("SUPER + Return", hl.dsp.exec_cmd(terminal))
hl.bind("SUPER + T", hl.dsp.exec_cmd(terminal))
hl.bind("SUPER + E", hl.dsp.exec_cmd(fileManager))
hl.bind("SUPER + B", hl.dsp.exec_cmd(browser))
hl.bind("SUPER + M", hl.dsp.exec_cmd(music))
hl.bind("SUPER + N", hl.dsp.exec_cmd(terminal .. " -e nvim"))
hl.bind("SUPER + G", hl.dsp.exec_cmd(messenger))
hl.bind("SUPER + O", hl.dsp.exec_cmd("obsidian"))
hl.bind("SUPER + Slash", hl.dsp.exec_cmd(passwordManager))

hl.bind("ALT + SPACE", hl.dsp.exec_cmd("vicinae toggle"))
hl.bind("SUPER + V", hl.dsp.exec_cmd("vicinae vicinae://launch/clipboard/history"))
hl.bind("SUPER + C", hl.dsp.exec_cmd("quickmenu"))
hl.bind("SUPER + SHIFT + SPACE", hl.dsp.exec_cmd("hyprfocus-toggle"))
hl.bind("CTRL + SHIFT + Escape", hl.dsp.exec_cmd("pkill waybar || waybar"))

hl.bind("SUPER + SHIFT + L", hl.dsp.exec_cmd("hyprlock"))
hl.bind("ALT + F4", hl.dsp.exec_cmd("powermenu"))

hl.bind("SUPER + A", hl.dsp.exec_cmd(webapp .. "https://chatgpt.com"))
hl.bind("SUPER + SHIFT + A", hl.dsp.exec_cmd(webapp .. "https://grok.com"))
hl.bind("SUPER + Y", hl.dsp.exec_cmd(webapp .. "https://youtube.com/"))
hl.bind("SUPER + SHIFT + G", hl.dsp.exec_cmd(webapp .. "https://web.whatsapp.com/"))
hl.bind("SUPER + X", hl.dsp.exec_cmd(webapp .. "https://x.com/"))
hl.bind("SUPER + SHIFT + X", hl.dsp.exec_cmd(webapp .. "https://x.com/compose/post"))

--------------------
----  TILING  ----
--------------------

-- Close window
hl.bind("SUPER + Q", hl.dsp.window.close())

-- Control tiling
hl.bind("SUPER + P", hl.dsp.window.pseudo())
hl.bind("SUPER + T", hl.dsp.window.float({ action = "toggle" }))
hl.bind("SUPER + F", hl.dsp.window.fullscreen({ action = "toggle" }))
hl.bind("SUPER + D", hl.dsp.window.fullscreen({ action = "toggle", mode = 1 }))

-- Move focus with Super + arrow keys
hl.bind("SUPER + Left", hl.dsp.focus({ direction = "left" }))
hl.bind("SUPER + Right", hl.dsp.focus({ direction = "right" }))
hl.bind("SUPER + Up", hl.dsp.focus({ direction = "up" }))
hl.bind("SUPER + Down", hl.dsp.focus({ direction = "down" }))

-- Switch workspaces with Super + [0-9]
-- Move active window to a workspace with Super + SHIFT + [0-9]
for i = 1, 10 do
  local key = i % 10 -- 10 maps to key 0
  hl.bind("SUPER + " .. key, hl.dsp.focus({ workspace = i }))
  hl.bind("SUPER + SHIFT + " .. key, hl.dsp.window.move({ workspace = i }))
end

-- Special workspace (scratchpad)
hl.bind("SUPER + S", hl.dsp.workspace.toggle_special("magic"))
hl.bind("SUPER + SHIFT + S", hl.dsp.window.move({ workspace = "special:magic" }))

-- Swap active window with the one next to it with mainMod + SHIFT + CTRL + arrow keys
hl.bind("SUPER + CTRL + Left", hl.dsp.window.swap({ direction = "left" }))
hl.bind("SUPER + CTRL + Right", hl.dsp.window.swap({ direction = "right" }))
hl.bind("SUPER + CTRL + Up", hl.dsp.window.swap({ direction = "up" }))
hl.bind("SUPER + CTRL + Down", hl.dsp.window.swap({ direction = "down" }))

-- Layout msg
hl.bind("SUPER + SHIFT + Left", hl.dsp.layout("addmaster"))
hl.bind("SUPER + SHIFT + Right", hl.dsp.layout("removemaster"))

-- Resize active window
hl.bind("SUPER + Minus", hl.dsp.window.resize({ x = -100, y = 0 }))
hl.bind("SUPER + Equal", hl.dsp.window.resize({ x = 100, y = 0 }))
hl.bind("SUPER + SHIFT + Minus", hl.dsp.window.resize({ x = 0, y = -100 }))
hl.bind("SUPER + SHIFT + Equal", hl.dsp.window.resize({ x = 0, y = 100 }))

-- Scroll through existing workspaces with Super + scroll
hl.bind("SUPER + mouse_down", hl.dsp.focus({ workspace = "e+1" }))
hl.bind("SUPER + mouse_up", hl.dsp.focus({ workspace = "e-1" }))

--- Move/resize windows with Super + LMB/RMB and dragging
hl.bind("SUPER + mouse:272", hl.dsp.window.drag(), { mouse = true })
hl.bind("SUPER + mouse:273", hl.dsp.window.resize(), { mouse = true })

-------------------
----  MEDIA   ----
-------------------

-- SwayOSD client for volume and brightness feedback
hl.bind(
  "XF86AudioRaiseVolume",
  hl.dsp.exec_cmd("swayosd-client --output-volume +2 --max-volume=100"),
  { locked = true, repeating = true }
)
hl.bind(
  "XF86AudioLowerVolume",
  hl.dsp.exec_cmd("swayosd-client --output-volume -2"),
  { locked = true, repeating = true }
)
hl.bind(
  "XF86AudioMute",
  hl.dsp.exec_cmd("swayosd-client --output-volume mute-toggle"),
  { locked = true }
)
hl.bind(
  "XF86AudioMicMute",
  hl.dsp.exec_cmd("swayosd-client --input-volume mute-toggle"),
  { locked = true }
)

hl.bind(
  "XF86MonBrightnessUp",
  hl.dsp.exec_cmd("swayosd-client --brightness raise 5%+"),
  { locked = true, repeating = true }
)
hl.bind(
  "XF86MonBrightnessDown",
  hl.dsp.exec_cmd("swayosd-client --brightness lower 5%-"),
  { locked = true, repeating = true }
)

-- Brightness presets
hl.bind("SUPER + F2", hl.dsp.exec_cmd("swayosd-client --brightness 100"), { locked = true })
hl.bind("SUPER + F3", hl.dsp.exec_cmd("swayosd-client --brightness 0"), { locked = true })

-- Media player controls via SwayOSD (for OSD feedback)
hl.bind("XF86AudioNext", hl.dsp.exec_cmd("swayosd-client --playerctl next"), { locked = true })
hl.bind(
  "XF86AudioPause",
  hl.dsp.exec_cmd("swayosd-client --playerctl play-pause"),
  { locked = true }
)
hl.bind(
  "XF86AudioPlay",
  hl.dsp.exec_cmd("swayosd-client --playerctl play-pause"),
  { locked = true }
)
hl.bind("XF86AudioPrev", hl.dsp.exec_cmd("swayosd-client --playerctl previous"), { locked = true })

-- Lock keys OSD feedback
hl.bind(
  "Caps_Lock",
  hl.dsp.exec_cmd("swayosd-client --caps-lock"),
  { locked = true, trigger = "release" }
)
hl.bind(
  "Scroll_Lock",
  hl.dsp.exec_cmd("swayosd-client --scroll-lock"),
  { locked = true, trigger = "release" }
)
hl.bind(
  "Num_Lock",
  hl.dsp.exec_cmd("swayosd-client --num-lock"),
  { locked = true, trigger = "release" }
)

-----------------------
----  UTILITIES   ----
-----------------------

-- Screenshots
hl.bind("SUPER + PRINT", hl.dsp.exec_cmd("hyprshot -m region"))
hl.bind("SUPER + SHIFT + PRINT", hl.dsp.exec_cmd("hyprshot -m window"))
hl.bind("SUPER + CTRL + PRINT", hl.dsp.exec_cmd("hyprshot -m output"))

-- Color picker
hl.bind("SUPER + ALT + P", hl.dsp.exec_cmd("hyprpicker -a"))

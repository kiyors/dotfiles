-- See https://wiki.hyprland.org/Configuring/Window-Rules/ for more
hl.window_rule({
  name = "suppress-maximize",
  match = { class = ".*" },
  suppress_event = "maximize"
})

-- Hyprland-run windowrule
hl.window_rule({
  name  = "move-hyprland-run",
  match = { class = "hyprland-run" },
  move  = "20 monitor_h-120",
  float = true,
})

-- Force google-chrome into a tile to deal with --app bug
hl.window_rule({ match = { class = "^(Google-chrome)$" }, tile = true })

-- Float and center sound, bluetooth, and wifi settings, as well as nautilus previews
hl.window_rule({
  match = { class = "^(org.pulseaudio.pavucontrol|blueberry.py|Impala|Bluetui|org.gnome.NautilusPreviewer)$" },
  float = true
})
hl.window_rule({
  match = { class = "^(org.pulseaudio.pavucontrol|blueberry.py|Impala|Bluetui|org.gnome.NautilusPreviewer)$" },
  size = { 800, 600 }
})
hl.window_rule({
  match = { class = "^(org.pulseaudio.pavucontrol|blueberry.py|Impala|Bluetui|org.gnome.NautilusPreviewer)$" },
  center = true
})

-- Float and center file pickers
hl.window_rule({
  match = { class = "xdg-desktop-portal-gtk", title = "^(Open.*Files?|Save.*Files?)" },
  float = true
})
hl.window_rule({
  match = { class = "xdg-desktop-portal-gtk", title = "^(Open.*Files?|Save.*Files?)" },
  center = true
})

-- Float Steam, fullscreen RetroArch
hl.window_rule({ match = { class = "^(steam)$" }, float = true })
hl.window_rule({ match = { class = "^(com.libretro.RetroArch)$" }, fullscreen = true })

-- Modal tags
hl.window_rule({ match = { tag = "modal" }, float = true, pin = true, center = true })

-- App specific rules
hl.window_rule({ match = { title = "^(Media viewer)$" }, float = true })
hl.window_rule({ match = { title = "^(.*Bitwarden Password Manager.*)$" }, float = true })
hl.window_rule({ match = { class = "^(org.gnome.Calculator)$" }, float = true, size = { 360, 490 } })
hl.window_rule({ match = { title = "^(Picture-in-Picture)$" }, float = true, pin = true })

-- Idle inhibit
hl.window_rule({ match = { class = "^(mpv|.+exe|celluloid)$" }, idle_inhibit = "focus" })
hl.window_rule({ match = { class = "^(zen)$", title = "^(.*YouTube.*)$" }, idle_inhibit = "focus" })
hl.window_rule({ match = { class = "^(zen)$" }, idle_inhibit = "fullscreen" })

-- Dim around
hl.window_rule({ match = { class = "^(gcr-prompter|xdg-desktop-portal-gtk|polkit-gnome-authentication-agent-1)$" }, dim_around = true })
hl.window_rule({ match = { class = "^(zen)$", title = "^(File Upload)$" }, dim_around = true })

-- Jetbrains
hl.window_rule({ match = { class = "^(.*jetbrains.*)$", title = "^(Confirm Exit|Open Project|win424|win201|splash)$" }, center = true })
hl.window_rule({ match = { class = "^(.*jetbrains.*)$", title = "^(splash)$" }, size = { 640, 400 } })

-- Just dash of opacity
hl.window_rule({ match = { class = ".*" }, opacity = "0.97 0.9" })
hl.window_rule({ match = { class = "^(google-chrome|google-chrome-unstable|firefox|zen)$" }, opacity = "1 0.97" })
hl.window_rule({ match = { initial_title = "^(youtube.com_/)$" }, opacity = "1 1" })
hl.window_rule({
  match = { class = "^(zoom|vlc|mpv|org.kde.kdenlive|com.obsproject.Studio|com.github.PintaProject.Pinta|imv)$" },
  opacity = "1 1"
})
hl.window_rule({ match = { class = "^(com.libretro.RetroArch|steam)$" }, opacity = "1 1" })

-- Fix some dragging issues with XWayland
hl.window_rule({
  name     = "fix-xwayland-drags",
  match    = {
    class      = "^$",
    title      = "^$",
    xwayland   = true,
    float      = true,
    fullscreen = false,
    pin        = false,
  },
  no_focus = true,
})

-- Workspace assignment
hl.window_rule({ match = { class = "^([Ss]potify)$" }, workspace = 1 })
hl.window_rule({ match = { class = "^([Gg]oogle-chrome|zen)$" }, workspace = 2 })
hl.window_rule({ match = { class = "^(com.mitchellh.ghostty|ghostty)$" }, workspace = 3 })

-- Layer rules
hl.layer_rule({ match = { namespace = "launcher" }, no_anim = true })
hl.layer_rule({ match = { namespace = "^ags-.*" }, no_anim = true })

-- vicinae
hl.layer_rule({
  match = { namespace = "vicinae" },
  name = "vicinae-blur",
  blur = true,
  ignore_alpha = 0,
})

hl.layer_rule({
  match = { namespace = "vicinae" },
  name = "vicinae-no-animation",
  no_anim = true,
})

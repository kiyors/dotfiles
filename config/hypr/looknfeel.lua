-- Refer to https://wiki.hyprland.org/Configuring/Variables/

-- https://wiki.hyprland.org/Configuring/Variables/#general
hl.config({
  general = {
    gaps_in = 3,
    gaps_out = 8,

    border_size = 1,

    -- https://wiki.hyprland.org/Configuring/Variables/#variable-types for info about colors
    col = {
      active_border = "rgba(ca9ee6ff)",
      inactive_border = "rgba(b4befecc)",
    },

    -- Set to true enable resizing windows by clicking and dragging on borders and gaps
    resize_on_border = true,

    -- Please see https://wiki.hyprland.org/Configuring/Tearing/ before you turn this on
    allow_tearing = false,

    layout = "master",
  },

  -- https://wiki.hyprland.org/Configuring/Variables/#decoration
  decoration = {
    rounding = 10,

    active_opacity = 1.0,
    inactive_opacity = 0.9,

    shadow = {
      enabled = true,
      range = 20,
      render_power = 3,
      color = "rgba(1a1a1aee)",
    },

    -- https://wiki.hyprland.org/Configuring/Variables/#blur
    blur = {
      enabled = true,
      size = 18,
      passes = 1,
      vibrancy = 0.1696,
    },
  },
})

-- https://wiki.hyprland.org/Configuring/Variables/#animations
hl.curve("linear", { type = "bezier", points = { { 0, 0 }, { 1, 1 } } })
hl.curve("md3_standard", { type = "bezier", points = { { 0.2, 0 }, { 0, 1 } } })
hl.curve("md3_decel", { type = "bezier", points = { { 0.05, 0.7 }, { 0.1, 1 } } })
hl.curve("md3_accel", { type = "bezier", points = { { 0.3, 0 }, { 0.8, 0.15 } } })
hl.curve("overshot", { type = "bezier", points = { { 0.05, 0.9 }, { 0.1, 1.1 } } })
hl.curve("crazyshot", { type = "bezier", points = { { 0.1, 1.5 }, { 0.76, 0.92 } } })
hl.curve("hyprnostretch", { type = "bezier", points = { { 0.05, 0.9 }, { 0.1, 1.0 } } })
hl.curve("menu_decel", { type = "bezier", points = { { 0.1, 1 }, { 0, 1 } } })
hl.curve("menu_accel", { type = "bezier", points = { { 0.38, 0.04 }, { 1, 0.07 } } })
hl.curve("easeInOutCirc", { type = "bezier", points = { { 0.85, 0 }, { 0.15, 1 } } })
hl.curve("easeOutCirc", { type = "bezier", points = { { 0, 0.55 }, { 0.45, 1 } } })
hl.curve("easeOutExpo", { type = "bezier", points = { { 0.16, 1 }, { 0.3, 1 } } })
hl.curve("softAcDecel", { type = "bezier", points = { { 0.26, 0.26 }, { 0.15, 1 } } })
hl.curve("md2", { type = "bezier", points = { { 0.4, 0 }, { 0.2, 1 } } })

hl.animation({ leaf = "global", enabled = true, speed = 10, bezier = "default" })
hl.animation({ leaf = "windows", enabled = true, speed = 1.5, bezier = "md3_decel", style = "popin 60%" })
hl.animation({ leaf = "windowsIn", enabled = true, speed = 1.5, bezier = "md3_decel", style = "popin 60%" })
hl.animation({ leaf = "windowsOut", enabled = true, speed = 1.5, bezier = "md3_accel", style = "popin 60%" })
hl.animation({ leaf = "border", enabled = true, speed = 3, bezier = "default" })
hl.animation({ leaf = "fade", enabled = true, speed = 1.5, bezier = "md3_decel" })
hl.animation({ leaf = "layersIn", enabled = true, speed = 1.5, bezier = "menu_decel", style = "slide" })
hl.animation({ leaf = "layersOut", enabled = true, speed = 1.5, bezier = "menu_accel" })
hl.animation({ leaf = "fadeLayersIn", enabled = true, speed = 1.5, bezier = "menu_decel" })
hl.animation({ leaf = "fadeLayersOut", enabled = true, speed = 1.5, bezier = "menu_accel" })
hl.animation({ leaf = "workspaces", enabled = true, speed = 1.5, bezier = "menu_decel", style = "slide" })
hl.animation({ leaf = "specialWorkspace", enabled = true, speed = 1.5, bezier = "md3_decel", style = "slidevert" })
hl.animation({ leaf = "zoomFactor", enabled = true, speed = 7, bezier = "default" })

-- See https://wiki.hypr.land/Configuring/Layouts/Dwindle-Layout/ for more
hl.config({
  dwindle = {
    preserve_split = true, -- You probably want this
  },
})

-- See https://wiki.hypr.land/Configuring/Layouts/Master-Layout/ for more
hl.config({
  master = {
    new_status = "master",
    allow_small_split = true,
    mfact = 0.5,
  },
})

-- See https://wiki.hypr.land/Configuring/Layouts/Scrolling-Layout/ for more
hl.config({
  scrolling = {
    fullscreen_on_one_column = true,
  },
})

hl.config({
  misc = {
    force_default_wallpaper = 1,
    disable_hyprland_logo = true,
    disable_autoreload = false,
    focus_on_activate = true,
  },
})

-- https://wiki.hyprland.org/Configuring/Variables/#input
hl.config({
  input = {
    kb_layout          = "us",
    kb_variant         = "",
    kb_model           = "",
    kb_options         = "",
    kb_rules           = "",

    follow_mouse       = 1,

    sensitivity        = 1, -- -1.0 - 1.0, 0 means no modification.
    repeat_delay       = 300,
    repeat_rate        = 50,
    numlock_by_default = true,

    touchpad           = {
      natural_scroll = true,
      clickfinger_behavior = true,
    },
  },
})

hl.gesture({
  fingers = 3,
  direction = "horizontal",
  action = "workspace"
})

-- Permissions for security
hl.permission("/usr/bin/grim", "screencopy", "allow")
hl.permission("/usr/lib/xdg-desktop-portal-hyprland", "screencopy", "allow")
hl.permission("/usr/bin/hyprpm", "plugin", "allow")

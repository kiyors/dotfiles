-- Cursor size
hl.env("XCURSOR_SIZE", "24")
hl.env("HYPRCURSOR_SIZE", "24")
hl.env("GDK_SCALE", "1")

-- Force all apps to use Wayland
hl.env("XDG_CURRENT_DESKTOP", "Hyprland")
hl.env("XDG_SESSION_TYPE", "wayland")
hl.env("XDG_SESSION_DESKTOP", "Hyprland")
hl.env("GDK_BACKEND", "wayland")
hl.env("QT_QPA_PLATFORM", "wayland;xcb")
hl.env("QT_AUTO_SCREEN_SCALE_FACTOR", "1")
hl.env("QT_WAYLAND_DISABLE_WINDOWDECORATION", "1")
hl.env("QT_STYLE_OVERRIDE", "kvantum")
hl.env("SDL_VIDEODRIVER", "wayland")
hl.env("CLUTTER_BACKEND", "wayland")
hl.env("MOZ_ENABLE_WAYLAND", "1")
hl.env("ANKI_WAYLAND", "1")
hl.env("NIXOS_OZONE_WL", "1")
hl.env("ELECTRON_OZONE_PLATFORM_HINT", "auto")

hl.env("WLR_BACKEND", "vulkan")
hl.env("WLR_RENDERER", "vulkan")
hl.env("WLR_NO_HARDWARE_CURSORS", "1")
hl.env("WLR_DRM_NO_ATOMIC", "1")

hl.env("__GL_GSYNC_ALLOWED", "0")
hl.env("__GL_VRR_ALLOWED", "0")
hl.env("DISABLE_QT5_COMPAT", "0")
hl.env("DIRENV_LOG_FORMAT", "")

-- AQ_DRM_DEVICES helps with multiple GPUs
hl.env("AQ_DRM_DEVICES", "/dev/dri/card2:/dev/dri/card1")

-- Make .desktop files available for launching apps from Hyprland
hl.env("XDG_DATA_DIRS", "/usr/share:/usr/local/share:~/.local/share")

hl.config({
  cursor = {
    no_hardware_cursors = true,
    default_monitor = "eDP-2"
  },
  xwayland = {
    force_zero_scaling = true
  },
  -- Don't show update on first launch
  ecosystem = {
    no_update_news = true
  }
})


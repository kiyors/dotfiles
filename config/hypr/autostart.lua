hl.on("hyprland.start", function ()
  hl.exec_cmd("dbus-update-activation-environment --systemd --all")
  hl.exec_cmd("systemctl --user start hyprpolkitagent")
  hl.exec_cmd("hypridle")
  hl.exec_cmd("systemctl --user start vicinae.service")
  hl.exec_cmd("waybar")
  hl.exec_cmd("swayosd-server")
  hl.exec_cmd("wl-clip-persist --clipboard regular --all-mime-type-regex '^(?!x-kde-passwordManagerHint).+'")
  hl.exec_cmd("hyprpaper")
  hl.exec_cmd("nextcloud --background")

  -- amane shell, opt-in. home/amane.nix exports AMANE_AUTOSTART=1 when
  -- desktop.amane.autostart is enabled, and symlinks this shell's source to
  -- dotfiles/config/amane. Left off by default because amane draws its own
  -- bar, notification daemon and lock screen, which would sit on top of
  -- waybar, swaync and hyprlock. `amane run` only starts the last
  -- `amane compile`.
  if os.getenv("AMANE_AUTOSTART") == "1" then
    hl.exec_cmd("amane run")
  end
end)

{
  myLib,
  config,
  inputs,
  pkgs,
  lib,
  ...
}:
let
  cfg = config.desktop.amane;

  # amane's flake publishes an x86_64-linux package only, so this module is a
  # no-op on the aarch64-darwin host.
  amaneSystem = "x86_64-linux";
  amaneSupported = pkgs.stdenv.hostPlatform.system == amaneSystem;
  amanePkg = inputs.amane.packages.${amaneSystem}.default;

  # The command copies the library out of itself and runs `cargo` on every
  # build, so the toolchain has to stay on PATH for as long as the config
  # lives. The package wrapper only covers the process it starts, not the
  # shell you run `amane compile` from.
  buildRequirements = with pkgs; [
    cargo
    rustc
    pkg-config
  ];

  # Hard requirements. Without these the shell renders but misbehaves:
  # `magick` drives floating-widget placement analysis, `curl` fetches
  # weather and album art.
  requiredTools = with pkgs; [
    imagemagick
    curl
  ];

  # One optional feature each. Every one of these is opt-in inside suzuha's
  # settings window, so they only matter once you switch a feature on.
  optionalTools = with pkgs; [
    cava # control-centre visualiser
    ffmpeg # image/video/audio conversion
    pulseaudio # pactl, for recording desktop + mic together
    wf-recorder # screen recording
    libnotify # notify-send, for pomodoro and recorder alerts
    pipewire # pw-play, for pomodoro chimes
    dconf # GTK theme switching
  ];

  # Everything below is gated on this one predicate: the module is enabled
  # *and* the host is a platform amane is packaged for. Gating only the
  # packages would still leak the autostart variable and the symlinks onto
  # Darwin, where nothing consumes them.
  active = cfg.enable && amaneSupported;

  autostarting = active && cfg.autostart;
in
# `myLib.mkHomeModule` only creates the `enable` flag, so the autostart opt-in
# is layered on afterwards as a sibling option.
myLib.mkHomeModule {
  globalConfig = config;
  name = "desktop.amane";
  description = "amane Rust desktop shell driven by the suzuha config";
  config = {
    # Referencing the package on Darwin would force an x86_64-linux nixpkgs
    # evaluation for a shell that could never run.
    home.packages = lib.optionals active (
      [ amanePkg ] ++ buildRequirements ++ requiredTools ++ optionalTools
    );

    # The shell source lives in the dotfiles checkout. It has to be an
    # out-of-store symlink: `amane compile` writes a generated Cargo.toml,
    # a Cargo.lock and target/ next to src/ on every build, which a read-only
    # store copy could not accept.
    #
    # optionalAttrs rather than mkIf: naming a submodule attribute at all
    # instantiates it, and an instantiated submodule with no `source` still
    # shows up in home.file. Declaring nothing keeps the host clean.
    xdg.configFile = lib.optionalAttrs active {
      "amane".source =
        config.lib.file.mkOutOfStoreSymlink "${config.home.homeDirectory}/dotfiles/config/amane";
    };

    # suzuha persists settings here and rewrites them in place rather than
    # swapping in a new file, so a symlink survives an Apply from the
    # settings window and the dotfiles checkout stays the source of truth.
    xdg.stateFile = lib.optionalAttrs active {
      "amane/settings".source =
        config.lib.file.mkOutOfStoreSymlink "${config.home.homeDirectory}/dotfiles/config/amane/settings";
    };

    # config/hypr/autostart.lua reads this and spawns `amane run`. The flag is
    # opt-in because amane draws its own bar, notification daemon and lock
    # screen, and those collide with waybar/swaync/hyprlock.
    home.sessionVariables = lib.optionalAttrs autostarting {
      AMANE_AUTOSTART = "1";
    };

    warnings =
      lib.optional (autostarting && config.desktop.waybar.enable)
        "desktop.amane.autostart is on and desktop.waybar is still enabled: both draw a top bar. Set desktop.waybar.enable = false to hand the bar over."
      ++
        lib.optional (autostarting && config.desktop.swaync.enable)
          "desktop.amane.autostart is on and desktop.swaync is still enabled: both claim org.freedesktop.Notifications and only one wins, so notifications may land in swaync while amane's notification panel stays empty. Set desktop.swaync.enable = false to hand the daemon over."
      ++
        lib.optional (autostarting && config.desktop.swayosd.enable)
          "desktop.amane.autostart is on and desktop.swayosd is still enabled: both draw a volume and brightness overlay. Set desktop.swayosd.enable = false to hand the overlay over.";
  };

  options.desktop.amane.autostart = myLib.mkBoolOpt false "Start the amane shell from Hyprland's autostart (needs AMANE_AUTOSTART in autostart.lua)";
}

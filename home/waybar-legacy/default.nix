{
  config,
  pkgs,
  ...
}:
{
  imports = [
    ./swaync.nix
    ./swayosd.nix
  ];

  wayland.windowManager.hyprland.settings.exec-once = [
    "waybar "
    "${pkgs.networkmanagerapplet}/bin/nm-applet"
    "${pkgs.blueman}/bin/blueman-applet"
  ];

  programs.waybar = {
    enable = true;
    systemd.enable = true;
    settings.mainBar = {
      layer = "top";
      position = "top";
      mode = "dock";
      height = 28;
      exclusive = true;
      passthrough = false;
      gtk-layer-shell = true;
      ipc = true;
      fixed-center = true;
      margin-top = 8;
      margin-left = 8;
      margin-right = 8;
      margin-bottom = 0;
      spacing = 5;

      modules-left = [
        "hyprland/workspaces"
        "cava"
      ];
      modules-center = [
        "idle_inhibitor"
        "clock"
      ];
      modules-right = [
        "cpu"
        "memory"
        "backlight"
        "pulseaudio"
        "battery"
        "tray"
        "custom/notification"
      ];

      "hyprland/workspaces" = {
        disable-scroll = true;
        all-outputs = true;
        active-only = false;
        on-click = "activate";
        persistent-workspaces = {
          "*" = [
            1
            2
            3
            4
            5
            6
            7
            8
            9
            10
          ];
        };
      };

      "hyprland/window" = {
        format = "  {}";
        separate-outputs = true;
        rewrite = {
          "^(.*?)[[:space:]]*[-—|].*?$" = "$1";
          "(.*) — Mozilla Firefox" = "$1 󰈹";
          "(.*)Mozilla Firefox" = " Firefox 󰈹";
          "(.*) - Visual Studio Code" = "$1 󰨞";
          "(.*)Visual Studio Code" = "Code 󰨞";
          "(.*) — Dolphin" = "$1 󰉋";
          "(.*)Spotify" = "Spotify 󰓇";
          "(.*)Steam" = "Steam 󰓓";
        };
        icon = true;
        icon-size = 20;
        max-length = 30;
      };

      cava = {
        hide_on_silence = false;
        framerate = 60;
        bars = 10;
        format-icons = [
          "▁"
          "▂"
          "▃"
          "▄"
          "▅"
          "▆"
          "▇"
          "█"
        ];
        input_delay = 1;
        sleep_timer = 5;
        bar_delimiter = 0;
        on-click = "playerctl play-pause";
      };

      "idle_inhibitor" = {
        format = "{icon}";
        format-icons = {
          activated = "󰥔";
          deactivated = "";
        };
      };

      clock = {
        format = "{:%a %d %b %R}";
        format-alt = "{:%I:%M %p}";
        timezone = "Asia/Kolkata";
        tooltip-format = "<tt>{calendar}</tt>";
        calendar = {
          mode = "month";
          mode-mon-col = 3;
          on-scroll = 1;
          on-click-right = "mode";
          format = {
            months = "<span color='#ffead3'><b>{}</b></span>";
            weekdays = "<span color='#ffcc66'><b>{}</b></span>";
            today = "<span color='#ff6699'><b>{}</b></span>";
          };
        };
        interval = 60;
        max-length = 25;
        on-click = "brave --profile-directory=Default --app-id=ojibjkjikcpjonjjngfkegflhmffeemk";
        actions = {
          on-click-right = "mode";
          on-click-forward = "tz_up";
          on-click-backward = "tz_down";
          on-scroll-up = "shift_up";
          on-scroll-down = "shift_down";
        };
      };

      cpu = {
        interval = 10;
        format = "󰍛 {usage}%";
        format-alt = "{icon0}{icon1}{icon2}{icon3}";
        format-icons = [
          "▁"
          "▂"
          "▃"
          "▄"
          "▅"
          "▆"
          "▇"
          "█"
        ];
      };

      memory = {
        interval = 30;
        format = "󰾆 {percentage}%";
        format-alt = "󰾅 {used}GB";
        max-length = 10;
        tooltip = true;
        tooltip-format = " {used:.1f}GB/{total:.1f}GB";
      };

      backlight = {
        format = "{icon}  {percent}%";
        format-icons = [
          "󰃞"
          "󰃟"
          "󰃠"
        ];
        on-scroll-up = "${pkgs.brightnessctl}/bin/brightnessctl set 2%+";
        on-scroll-down = "${pkgs.brightnessctl}/bin/brightnessctl set 2%-";
      };

      pulseaudio = {
        format = "{icon}";
        format-muted = "󰖁";
        format-icons = {
          headphone = "";
          hands-free = "";
          headset = "";
          phone = "";
          portable = "";
          car = "";
          default = [
            "󰕿"
            "󰖀"
            "󰕾"
          ];
        };
        on-click = "sound-toggle";
        on-click-right = "pavucontrol -t 3";
        tooltip-format = "{icon} {desc} // {volume}%";
        scroll-step = 1;
      };

      battery = {
        states = {
          good = 95;
          warning = 30;
          critical = 20;
        };
        format = "{icon}  {capacity}%";
        format-charging = " {capacity}%";
        format-plugged = " {capacity}%";
        format-alt = "{time} {icon}";
        format-icons = {
          default = [
            "󰁺"
            "󰁻"
            "󰁼"
            "󰁽"
            "󰁾"
            "󰁿"
            "󰂀"
            "󰂁"
            "󰂂"
            "󰁹"
          ];
          charging = [
            "󰢜"
            "󰂆"
            "󰂇"
            "󰂈"
            "󰢝"
            "󰂉"
            "󰢞"
            "󰂊"
            "󰂋"
            "󰂅"
          ];
        };
        format-full = "󱟢";
        on-click = "powermode-toggle";
        on-click-right = ''${pkgs.swayosd}/bin/swayosd-client --custom-message="Powermode is set to $(powerprofilesctl get)" --custom-icon="emblem-default"'';
      };

      "custom/notification" = {
        tooltip = false;
        format = "{icon}";
        format-icons = {
          notification = "<span foreground='#89b4fa'><sup></sup></span>"; # Blue color
          none = "";
          dnd-notification = "<span foreground='#89b4fa'><sup></sup></span>";
          dnd-none = "";
          inhibited-notification = "<span foreground='#89b4fa'><sup></sup></span>";
          inhibited-none = "";
          dnd-inhibited-notification = "<span foreground='#89b4fa'><sup></sup></span>";
          dnd-inhibited-none = "";
        };
        return-type = "json";
        exec-if = "which swaync-client";
        exec = "swaync-client -swb";
        on-click = "swaync-client -t";
        escape = true;
      };

      tray = {
        icon-size = 16;
        spacing = 8;
        show-passive-items = true;
      };
    };

    style = ''
      * {
        font-size: 13px;
        font-feature-settings: '"zero", "ss01", "ss02", "ss03", "ss04", "ss05", "cv31"';
        margin: 0px;
        padding: 0px;
      }

      @define-color base   #1e1e2e;
      @define-color mantle #181825;
      @define-color crust  #11111b;
      @define-color text     #cdd6f4;
      @define-color subtext0 #a6adc8;
      @define-color subtext1 #bac2de;
      @define-color surface0 #313244;
      @define-color surface1 #45475a;
      @define-color surface2 #585b70;
      @define-color overlay0 #6c7086;
      @define-color overlay1 #7f849c;
      @define-color overlay2 #9399b2;
      @define-color blue      #89b4fa;
      @define-color lavender  #b4befe;
      @define-color sapphire  #74c7ec;
      @define-color sky       #89dceb;
      @define-color teal      #94e2d5;
      @define-color green     #a6e3a1;
      @define-color yellow    #f9e2af;
      @define-color peach     #fab387;
      @define-color maroon    #eba0ac;
      @define-color red       #f38ba8;
      @define-color mauve     #cba6f7;
      @define-color pink      #f5c2e7;
      @define-color flamingo  #f2cdcd;
      @define-color rosewater #f5e0dc;

      window#waybar {
        transition-property: background-color;
        transition-duration: 0.5s;
        background: transparent;
        border-radius: 8px;
      }

      window#waybar.hidden {
        opacity: 0.2;
      }

      tooltip {
        background: #1e1e2e;
        border-radius: 6px;
      }

      tooltip label {
        color: #cad3f5;
        margin-right: 4px;
        margin-left: 4px;
      }

      .modules-left {
        background: @base;
        border: 1px solid @blue;
        padding-right: 12px;
        padding-left: 2px;
        border-radius: 8px;
      }

      .modules-center {
        background: @base;
        border: 0.5px solid @overlay0;
        padding-right: 4px;
        padding-left: 4px;
        border-radius: 8px;
      }

      .modules-right {
        background: @base;
        border: 1px solid @blue;
        padding-right: 12px;
        padding-left: 12px;
        border-radius: 8px;
      }

      #backlight,
      #battery,
      #clock,
      #cpu,
      #idle_inhibitor,
      #memory,
      #pulseaudio,
      #tray,
      #window,
      #workspaces,
      #cava,
      #custom-notification {
        padding-top: 2px;
        padding-bottom: 2px;
        padding-right: 5px;
        padding-left: 5px;
      }

      #idle_inhibitor {
        color: @blue;
      }

      #backlight {
        color: @blue;
      }

      #battery {
        color: #86a381;
        font-size: 16px;
        padding-left: 12px;
        padding-right: 12px;
        border-radius: 12px;
      }

      @keyframes blink {
        to {
          background-color: #f9e2af;
          color: #96804e;
        }
      }

      #battery.critical:not(.charging) {
        background-color: #f38ba8;
        color: #bf5673;
        animation-name: blink;
        animation-duration: 0.5s;
        animation-timing-function: linear;
        animation-iteration-count: infinite;
        animation-direction: alternate;
      }

      #clock {
        color: @yellow;
      }

      #cpu {
        color: @yellow;
      }

      #memory {
        color: @green;
      }

      #tray {
        background-color: transparent;
        padding-left: 8px;
        padding-right: 8px;
      }

      #tray > .passive {
        -gtk-icon-effect: none;
      }

      #tray > .needs-attention {
        -gtk-icon-effect: highlight;
      }

      /* Style tray icons with colors */
      #tray > widget > * {
        color: @blue;
      }

      #workspaces button {
        box-shadow: none;
        text-shadow: none;
        padding: 0px;
        border-radius: 8px;
        padding-left: 4px;
        padding-right: 4px;
        transition: all 0.5s cubic-bezier(.55,-0.68,.48,1.682);
        border: 0px solid transparent;  /* Remove any border completely */
      }

      #workspaces button:hover {
        border-radius: 8px;
        color: @overlay0;
        background-color: @surface0;
        padding-left: 2px;
        padding-right: 2px;
        transition: all 0.3s cubic-bezier(.55,-0.68,.48,1.682);
        border: 0px solid transparent;  /* Remove any border completely */
      }

      #workspaces button.persistent {
        color: @surface1;
        border-radius: 8px;
        border: 0px solid transparent;  /* Remove any border completely */
      }

      #workspaces button.active {
        color: @peach;
        border-radius: 8px;
        padding-left: 6px;
        padding-right: 6px;
        transition: all 0.3s cubic-bezier(.55,-0.68,.48,1.682);
        border: 0px solid transparent;  /* Remove any border completely */
      }

      #workspaces button.urgent {
        color: @red;
        border-radius: 8px;
        border: 0px solid transparent;  /* Remove any border completely */
      }

      #cava {
        color: @pink;
      }

      #custom-notification {
        color: @blue;           /* Blue color for notification icon */
        font-size: 18px;
        font-weight: bolder;
        padding-left: 16px;
        padding-right: 18px;
        border-radius: 12px;
      }

      #pulseaudio,
      #pulseaudio.muted {
        color: @blue;
        font-size: 18px;
        font-weight: bolder;
        padding-left: 12px;
        padding-right: 13px;
        border-radius: 12px;
      }

      #window {
        color: @mauve;
      }
    '';
  };
}

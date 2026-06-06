-- Learn how to configure Hyprland: https://wiki.hyprland.org/Configuring/

-- Change your personal monitor setup in here to keep the main config portable
require("monitors")

require("autostart")
require("keybindings")

require("envs")
require("looknfeel")
require("windows")

-- Custom layouts (can be enabled by uncommenting)
-- require("layouts.columns")
-- require("layouts.grid")
-- require("layouts.manual")
-- require("layouts.spiral")

-- Example per-device config
-- hl.device({
--     name        = "epic-mouse-v1",
--     sensitivity = -0.5,
-- })

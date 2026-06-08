{
  myLib,
  config,
  pkgs,
  lib,
  ...
}:
let
  inherit (config.home) homeDirectory;
  dotfiles = "${homeDirectory}/dotfiles";

  iconPackage = pkgs.whitesur-icon-theme.override {
    boldPanelIcons = true;
    alternativeIcons = true;
  };
in
myLib.mkHomeModule {
  globalConfig = config;
  name = "desktop.thunar";
  description = "Thunar file manager with custom configurations";
  config = lib.mkIf pkgs.stdenv.isLinux {
    home.packages = with pkgs; [
      thunar
      xfconf
      tumbler
      thunar-archive-plugin
      thunar-volman
      thunar-media-tags-plugin
      p7zip
      xarchiver
      gvfs # For trash and network mounting support
      ffmpegthumbnailer # For video thumbnails
    ];

    gtk = {
      iconTheme = {
        name = "WhiteSur";
        package = iconPackage;
      };
      gtk3.bookmarks = [
        "file://${homeDirectory}/Downloads Downloads"
        "file://${homeDirectory}/Pictures Pictures"
        "file://${homeDirectory}/dotfiles dotfiles"
        "file://${homeDirectory}/workspace workspace"
        "file://${homeDirectory}/personal personal"
        "file://${homeDirectory}/personal/media media"
        "file://${homeDirectory}/personal/projects projects"
      ];
    };

    home.sessionVariables = {
      XDG_ICON_DIR = "${iconPackage}/share/icons/WhiteSur";
    };

    home.file = {
      # Thunar Preferences (via Xfconf)
      ".config/xfce4/xfconf/xfce-perchannel-xml/thunar.xml".source =
        config.lib.file.mkOutOfStoreSymlink "${dotfiles}/config/thunar/thunar.xml";

      # Xarchiver Settings
      ".config/xarchiver/xarchiverrc".text = ''
        [xarchiver]
        preferred_format=0
        prefer_unzip=true
        confirm_deletion=true
        sort_filename_content=false
        advanced_isearch=true
        auto_expand=true
        store_output=false
        icon_size=2
        show_archive_comment=false
        show_sidebar=true
        show_location_bar=true
        show_toolbar=true
        preferred_custom_cmd=
        preferred_temp_dir=/tmp
        preferred_extract_dir=${homeDirectory}/Downloads
        allow_sub_dir=0
        ensure_directory=true
        overwrite=false
        full_path=2
        touch=false
        fresh=false
        update=false
        store_path=false
        updadd=true
        freshen=false
        recurse=true
        solid_archive=false
        remove_files=false
      '';
    };

    # Thunar Custom Actions
    xdg.configFile."Thunar/uca.xml".source =
      config.lib.file.mkOutOfStoreSymlink "${dotfiles}/config/thunar/uca.xml";
  };
}

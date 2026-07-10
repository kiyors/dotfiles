{ pkgs, rustPlatform, ... }:

rustPlatform.buildRustPackage {
  pname = "sys-clean";
  version = "0.1.0";

  src = ./sys-clean;

  cargoLock = {
    lockFile = ./sys-clean/Cargo.lock;
  };

  nativeBuildInputs = [ pkgs.installShellFiles ];

  postInstall = ''
    installShellCompletion --cmd sys-clean \
      --bash <($out/bin/sys-clean generate-completion bash) \
      --zsh <($out/bin/sys-clean generate-completion zsh) \
      --fish <($out/bin/sys-clean generate-completion fish)
  '';

  meta = with pkgs.lib; {
    description = "Custom system maintenance utility for macOS to reap orphaned Node processes and run GC";
    homepage = "https://github.com/gaurav/dotfiles";
    license = licenses.mit;
    maintainers = [ ];
    mainProgram = "sys-clean";
  };
}

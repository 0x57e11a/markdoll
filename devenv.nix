{ pkgs, lib, config, inputs, ... }: {
	packages = [ pkgs.git ];

	languages.rust.enable = true;
	languages.rust.channel = "nightly";
}

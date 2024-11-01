{ pkgs, lib, config, inputs, ... }: {
	packages = [ pkgs.git ];

	languages.rust.enable = true;
	languages.rust.channel = "nightly";

	enterTest = ''
		rm spec.html
		echo DEBUG
		cargo test -F ariadne
		echo RELEASE
		cargo test -F ariadne --release
	'';
}

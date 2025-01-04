{ pkgs, lib, config, inputs, ... }: {
	packages = [ pkgs.git ];

	languages.rust.enable = true;
	languages.rust.channel = "nightly";

	enterTest = ''
		rm spec.html
		echo DEBUG
		cat spec.doll | cargo run -F cli convert > spec.html
		echo RELEASE
		cat spec.doll | cargo run -F cli --release convert > spec.html
	'';
}

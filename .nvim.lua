-- NOTE: This requires vim.opt.exrc = true
local nvim_lsp = require("lspconfig")

nvim_lsp.rust_analyzer.setup({
	settings = {
		["rust-analyzer"] = {
			cargo = {
				target = "thumbv7em-none-eabi",
			},
			check = {
				allTargets = false,
				extraArgs = {
					"--target",
					"thumbv7em-none-eabi",
					"--bins",
				},
			},
			-- linkedProjects = {
			-- 	"notecard/Cargo.toml",
			-- 	"examples/swan/Cargo.toml",
			-- 	"examples/custom-stm32l4/Cargo.toml",
			-- },
		},
	},
})

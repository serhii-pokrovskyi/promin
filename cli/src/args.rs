use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, arg_required_else_help = true)]
pub struct CliArgs;

impl CliArgs {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}

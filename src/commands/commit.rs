#[derive(clap::Args)]
pub struct Args {
    pub msg: String,
}

pub fn run(args: Args) {
    println!("Commit : {}", args.msg);
}

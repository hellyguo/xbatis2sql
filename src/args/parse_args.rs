use getopts::*;
use std::env;

macro_rules! fail {
    ($f:tt, $o:tt) => {{
        eprintln!("Error: {}", $f);
        eprintln!();
        return Args::fail($o);
    }};
}

pub enum Mode {
    NotSupported,
    IBatis,
    MyBatis,
}

pub struct Args {
    pub mode: Mode,
    pub src_dir: String,
    pub output_dir: String,
    pub fast_fail: bool,
    opts: Options,
}

impl Args {
    fn new(mode: Mode, src_dir: &String, output_dir: &String, opts: Options) -> Self {
        return Args {
            mode: mode,
            src_dir: src_dir.clone(),
            output_dir: output_dir.clone(),
            opts: opts,
            fast_fail: false,
        };
    }

    fn fail(opts: Options) -> Self {
        return Args {
            mode: Mode::NotSupported,
            src_dir: String::from(""),
            output_dir: String::from(""),
            opts: opts,
            fast_fail: true,
        };
    }
}

/// 检查参数
pub fn check_args() -> Args {
    let opts = build_opts();
    let args: Vec<String> = env::args().collect();
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => {
            fail!(f, opts);
        }
    };
    let help = matches.opt_present("h");
    let mode_ibatis = matches.opt_present("i");
    let mode_mybatis = matches.opt_present("m");
    let src_dir = matches.opt_str("s");
    let output_dir = matches.opt_str("o");
    if help {
        return Args::fail(opts);
    } else if mode_ibatis && mode_mybatis {
        fail!("just support in mode: iBATIS or MyBatis, not both", opts);
    } else if !mode_ibatis && !mode_mybatis {
        fail!("must choose in iBATIS mode or MyBatis mode", opts);
    } else if src_dir.is_none() {
        fail!("must define the source directory", opts);
    } else if output_dir.is_none() {
        fail!("must define the output directory", opts);
    }
    if mode_ibatis {
        return Args::new(Mode::IBatis, &src_dir.unwrap(), &output_dir.unwrap(), opts);
    } else {
        return Args::new(Mode::MyBatis, &src_dir.unwrap(), &output_dir.unwrap(), opts);
    }
}

fn build_opts() -> Options {
    let mut opts = Options::new();
    opts.optflag("i", "ibatis", "try to parse iBATIS sqlmap files");
    opts.optflag("m", "mybatis", "try to parse MyBatis mapper files");
    opts.optopt("s", "src", "source directory", "SRC");
    opts.optopt("o", "output", "output directory", "OUTPUT");
    opts.optflag("h", "help", "print this help menu");
    return opts;
}

/// 打印使用方法
pub fn print_usage(args: &Args) {
    print!(
        "{}",
        args.opts.usage("Usage: xbatis2sql [-i|-m] -s ... -o ...")
    );
}

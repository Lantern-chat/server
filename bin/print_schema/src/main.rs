#[derive(Debug)]
pub struct CliOptions {
    pub db: String,
    pub out: Option<String>,
    pub schema: Option<String>,
}

static HELP: &str = r#"
Print Schema

USAGE:
    print_schema -d <db_str> [OPTIONS]

FLAGS:
    -h, --help      Prints help information

REQUIRED:
    -d, --db  <db_str>  PostgreSQL database connection string

OPTIONS:
    -o, --out <path>    Where to store generated schema file
    -s, --schema <schema>   Specific database schema to use
"#;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = {
        let mut pargs = pico_args::Arguments::from_env();

        if pargs.contains(["-h", "--help"]) {
            print!("{}", HELP);
            return Ok(());
        }

        CliOptions {
            out: pargs.opt_value_from_str(["-o", "--out"])?,
            db: pargs.value_from_str(["-d", "--db"])?,
            schema: pargs.opt_value_from_str(["-s", "--schema"])?,
        }
    };

    use db::pool::{Object, Pool};

    let pool = Pool::new(args.db.parse()?, db::pg::NoTls);
    let db = Object::take(pool.get().await?);

    let schema = thorn::generate::generate(db.as_ref(), args.schema.clone()).await?;

    if let Some(out) = args.out {
        use std::io::Write;

        let mut file = std::fs::OpenOptions::new().truncate(true).write(true).open(out)?;

        match args.schema {
            Some(schema) => write!(file, "//! Autogenerated Schema for \"{}\"\n\n", schema)?,
            None => write!(file, "//! Autogenerated Schema")?,
        }

        file.write(schema.as_bytes())?;
    } else {
        println!("{}", schema);
    }

    Ok(())
}

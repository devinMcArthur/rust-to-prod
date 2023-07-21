## Run Locally

- Read `cargo/config.toml` for installation of linkers

- `cargo watch -x run` to run the program

### Local Database

- `./scripts/init_db.sh`

## Deployment / Docker

### SQLx

Must first prepare sqlx for compile checks

`cargo sqlx prepare -- --lib`

#### Migrations

To create migration file
`sqlx migrate add <migration_name>`

To migrate local DB
`SKIP_DOCKER=true ./scripts/init_db.sh`

To migrate Production DB
`DATABASE_URL=postgres://${DB_USER}:${DB_PASSWORD}@${DB_HOST}:${DB_PORT}/${DB_NAME} sqlx migrate run`

## Tools

### cargo-expand

Used to view the code generated by macros

`cargo expand`

_Note `cargo +nightly expand` can be used to use the nightly toolchain for this single command, a feature of cargo_

### rstest

Parameterized testing

### sqlx-cli

Command Line Interface to manage database migrations

`cargo install --version="~0.6" sqlx-cli --no-default-features --features rustls,postgres`

### cargo-udeps

Scans Cargo.toml and checks if all crates are actually being used

`cargo install cargo-udeps`

`cargo +nightly udeps`

### bunyan

Prettifies outputted JSON

`cargo install bunyan`

`TEST_LOG=true cargo test | bunyan`

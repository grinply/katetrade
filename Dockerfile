.gitFROM rust:1.83-slim as build

# create a new empty shell project
RUN USER=root cargo new --bin katetrade
WORKDIR /katetrade

# copy over your manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

# this build step will cache your dependencies
RUN cargo build --release
RUN rm src/*.rs

# copy your source tree
COPY ./src ./src

# build for release
RUN rm ./target/release/deps/katetrade*
RUN cargo build --release

# our final base
FROM rust:1.83-slim

# copy the build artifact from the build stage
COPY --from=build /katetrade/target/release/katetrade .

# set the startup command to run your binary
ENTRYPOINT ["./katetrade"]



# Tinydancer (V0)

## Architecture
The client is designed to be modular and easily extensible for additional functionality as this is the v0 and with every major version there may be a need for major changes and add-on services.

### Bird's Eye View

![Architecture Diagram](https://res.cloudinary.com/dev-connect/image/upload/v1675235495/diet-client-v0-arch_bhdd4c.png)

## Getting started

Build and add to path
```
cargo b -r && cp ./target/release/tinydancer ~/.local/bin/
```
Or install using cargo
```
cargo install --git https://github.com/tinydancer-io/half-baked-client tinydancer
```
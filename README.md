# Tinydancer (V0)
<!-- ALL-CONTRIBUTORS-BADGE:START - Do not remove or modify this section -->
[![All Contributors](https://img.shields.io/badge/all_contributors-2-orange.svg?style=flat-square)](#contributors-)
<!-- ALL-CONTRIBUTORS-BADGE:END -->

## Architecture
The client is designed to be modular and easily extensible for additional functionality as this is the v0 and with every major version there may be a need for major changes and add-on services.

### Bird's Eye View

![Architecture Diagram](https://res.cloudinary.com/dev-connect/image/upload/v1675235495/diet-client-v0-arch_bhdd4c.png)

## Getting started
**Install Rust**
MacOS & Linux
```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
Note: For Windows download rustup from this [link](https://forge.rust-lang.org/infra/other-installation-methods.html#other-ways-to-install-rustup)

**Configure Toolchain**
```
rustup default nightly
```

**Build and Add to Path**
```
cargo b -r && cp ./target/release/tinydancer ~/.local/bin/
```
**Or Install Using Cargo**
```
cargo install --git https://github.com/tinydancer-io/half-baked-client tinydancer
```
**Confirm Installation**
```
tinydancer --help
```
## Testing
Testing is mostly manual, in the future we will implement unit tests 
but for now we have bash scripts in the `scripts` folder.

# Credits
The `rpc_wrapper` section of the client used to send rpc requests is borrowed from [blockworks-foundation/lite-rpc](https://github.com/blockworks-foundation/lite-rpc) and we are grateful to their team and the blockworks foundation for their work on it.

# Contributors

<!-- ALL-CONTRIBUTORS-LIST:START - Do not remove or modify this section -->
<!-- prettier-ignore-start -->
<!-- markdownlint-disable -->
<table>
  <tbody>
    <tr>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/0xNineteen"><img src="https://avatars.githubusercontent.com/u/100000306?v=4?s=100" width="100px;" alt="x19"/><br /><sub><b>x19</b></sub></a><br /><a href="https://github.com/tinydancer-io/half-baked-client/commits?author=0xNineteen" title="Code">💻</a></td>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/picklerick2349"><img src="https://avatars.githubusercontent.com/u/119039386?v=4?s=100" width="100px;" alt="Pickle Rick"/><br /><sub><b>Pickle Rick</b></sub></a><br /><a href="https://github.com/tinydancer-io/half-baked-client/commits?author=picklerick2349" title="Code">💻</a></td>
    </tr>
  </tbody>
</table>

<!-- markdownlint-restore -->
<!-- prettier-ignore-end -->

<!-- ALL-CONTRIBUTORS-LIST:END -->
<!-- prettier-ignore-start -->
<!-- markdownlint-disable -->

<!-- markdownlint-restore -->
<!-- prettier-ignore-end -->

<!-- ALL-CONTRIBUTORS-LIST:END -->

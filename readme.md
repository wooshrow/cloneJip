## How to install (on windows)
1. Install rust & cargo
2. Install cmake 
3. Install llvm
4. run `cargo build -vv` in the root folder (it can take up to 20 min for the z3-sys package to install, add the `-vv` flag to make sure the build process is still in progress)

## Generating tests from OOX programs


## Todo's
- [ ] experiment with lambda functions to shorten z3 module
- [ ] implement verification from files, and do tests using file loading
- [ ] put all error msgs in one or more enums to 'streamline' them
- [ ] linter installeren
- [ ] best practices opzoeken voor variable shadowing
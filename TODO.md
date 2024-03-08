- command: `install <product> --tandem`
  - should check related products, and install versions that work together
    - studio 5.2 == hubkit 5.2

- command: `install <product> --published`
  - should install the currently released master from an authoratative source
    (gravio.com)

- `--skip-cache`
  - ignores cached items, downloads from server always

- `StandaloneExe`
  - launchArgs etc valid for the metadata but this option goes nowhere. Also
    want to implement `stop_args`, `start_args`, and `run_as_service`

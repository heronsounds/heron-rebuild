# [0.2.0]

## Added
- Errors in traversal creation/workflow prep are stored and listed at end
  rather than failing immediately.
- Plans can have multiple goals, multiple branches, and span multiple lines.
- Can now run an anonymous plan specified on the command line with '-t' and '-b' flags.
- Branch identifiers can now start with a number.
- Branchpoint specification in values can now span multiple lines.
- Many new types of errors.
- Workflows can contain up to 128 branches.
- 3 Verbosity levels instead of simple true/false.

## Changed
- `GetStr`/`InternStr` traits now return `Result` instead of panicking.

## Fixed
- IOError when running in dry-run mode and trying to access `branchpoints.txt`.
- Opaque error when module dir doesn't exist
  (now waits to see if dir is actually used before erroring).
- Bug where duplicate tasks were sometimes being added to workflow run.


# [0.1.0]

- Initial version
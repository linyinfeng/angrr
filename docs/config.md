angrr 5
=======

# NAME

angrr - configuration file

# DESCRIPTION

Angrr configuration file is written in [TOML](https://toml.io) format. The global configuration file is located at `/etc/angrr/config.toml`. You can also specify a custom configuration file using the `--config` command line option.

# EXAMPLE

Run `angrr example-config` to extract the example configuration from the `angrr` binary.

```toml
EXAMPLE_CONFIG_PLACEHOLDER
```

# OPTIONS

**store** = \<path\>
:   Store path for validation. Default is `/nix/store`.

    Only GC roots pointing to store will be monitored.

**owned-only** = `"auto"`|`"true"`|`"false"`
:   Only monitors owned symbolic link target of GC roots.

    - `"auto"`: behaves like true for normal users, false for root.
    - `"true"`: only monitor GC roots owned by the current user.
    - `"false"`: monitor all GC roots.

**remove-root** = \<bool\>
:   Remove GC root in **directory** instead of the symbolic link target of them.

    Default is `false`.

**directory** = [\<path1\>, \<path2\>, ...]
:   Directories containing auto GC roots.

    Default is `["/nix/var/nix/gcroots/auto"]`.

**temporary-root-policies** = [\<policy1\>, \<policy2\>, ...]
:   List of temporary root policies.

    See **TEMPORARY ROOT POLICY OPTIONS** for details.

**profile-policies** = [\<policy1\>, \<policy2\>, ...]
:   List of profile policies.

    See **PROFILE POLICY OPTIONS** for details.

**touch** = \<touch-options\>
:   Options for `angrr touch` command.

    See **TOUCH OPTIONS** for details.

# COMMON POLICY OPTIONS

**enable** = \<bool\>
:   Enable or disable this policy. Default is `true`.

# TEMPORARY ROOT POLICY OPTIONS

See **COMMON POLICY OPTIONS** for common options.

**priority** = \<int\>
:   Priority of this policy.

    Lower number means higher priority, if multiple policies monitor the
    same path, the one with higher priority will be applied.
    If multiple policies have the same priority, name in lexicographical
    order will be applied. That is, a policy named "A" with priority 100
    will have higher priority than a policy named "B" with priority 100.

**path-regex** = \<regex\>
:   Only paths (absolute) matching the regex will be monitored by this policy.

**filter** = \<filter\>
:   An external program to filter paths that will be applied
    after all the other filter options.

    A JSON object containing the path information will be passed to the
    stdin of the program. If the program exits with code 0, then the
    path will be monitored; otherwise it will be ignored.

    See **FILTER OPTIONS** for more information.

**ignore-prefixes** = [\<prefix1\>, \<prefix2\>, ...]
:   A list of path prefixes (absolute) to ignore.

    Default is `["/nix/var/nix/profiles"]`.

**ignore-prefixes-in-home** = [\<prefix1\>, \<prefix2\>, ...]
:   A list of path prefixes (relative to home directory) to ignore.

    Default is:

    ```
    [
        ".local/state/nix/profiles",
        ".local/state/home-manager/gcroots",
        ".cache/nix/flake-registry.json"
    ]
    ```

**period** = \<duration\>
:   Retention period for temporary GC roots.

# PROFILE POLICY OPTIONS

Interaction between `keep-since` / `keep-latest-n` / `keep-n-per-bucket` / `keep-*` (etc) options:
There's no interaction; they don't depend on each other's values.
Multiple such rules may independently mark the same generation to be retained.

See **COMMON POLICY OPTIONS** for common options.

**profile-paths** = [\<path1\>, \<path2\>, ...]
:   Paths to the profile.

    When `owned-only = true`, if the option begins with `~`,
    it will be expanded to the home directory of the current user.

    When `owned-only = false`, if the option begins with `~`,
    it will be expanded to the home of all users discovered respectively.

**keep-since** = \<duration\>
:   Keep generations created within this duration.

**keep-latest-n** = \<number\>
:   Keep the latest \<number\> GC roots in this profile.

**keep-current-system** = \<bool\>
:   Whether to keep the current activated system generation.

    Only useful for system profiles. Default is `false`.

**keep-booted-system** = \<bool\>
:   Whether to keep the currently booted generation.

    Only useful for system profiles. Default is `false`.

**keep-n-per-bucket** = list of { n: \<usize\>, bucket-window: \<duration\>, bucket-amount: \<u32\> }
:   Specify a list of rules having `n`, `bucket-window`, and `bucket-amount` attributes.

    `n` defaults to 1.

    Each rule retains up to `n` generations every `bucket-window` duration for `bucket-amount` buckets,
    allowing generations to be kept over a long period with sparser older generations.
    This is useful when access to old generations is wanted,
    but keeping every generation in between is considered a waste of space.

    This attribute is inspired by the
    [grandfather-father-son](https://en.wikipedia.org/wiki/Backup_rotation_scheme#Grandfather-father-son)
    backup scheme.

    Take the configuration `{ n = 1; bucket-window = "1 Month"; bucket-amount = 2; }` as an example.
    Angrr will create two logical buckets spanning `[now to -30 days)` and `[-30 days to -60 days)`,
    each with a maximum capacity of 1 (`n`) generation.

    When specifying multiple rules, they will overlap with each other.
    Overlapping buckets/rules claim order is determined as follows:

    - Rules are checked in reverse order (from the last rule in `keep-n-per-bucket` to the first).
    - Within each bucket, older generations claim free slots first.

    Therefore, the order in which rules are declared matters:
    rules should be ordered from smallest `bucket-window` to largest,
    otherwise they may not work as expected.

    Whenever a generation matches a bucket with an available slot, it is retained
    and claims 1 of the bucket's `n` slots, making it unavailable to subsequent buckets and rules.
    Once a bucket's available slots reach 0, it cannot claim additional generations.
    When all buckets have been processed, any remaining generations not marked for retention
    by other `keep-*` rules will be deleted.

    `bucket-window` uses the `humantime::parse_duration` format, specified in the [DURATION section](#DURATION).
    `bucket-window` is relative to the moment angrr is run and does not follow calendar boundaries.
    For example, a period of one week represents the seven days before the run time, not the number
    of days since the start of the calendar week.

# FILTER OPTIONS

**program** = \<path\>
:   Path to the external filter program.

**arguments** = [\<arg1\>, \<arg2\>, ...]
:   Arguments to pass to the external filter program.

# TOUCH OPTIONS

**project-globs** = [\<glob1\>, \<glob2\>, ...]
:   List of glob patterns to include or exclude files when touching GC roots.

    Only applied when `angrr touch` is invoked with the `--project` flag.
    Patterns use an inverted gitignore-style semantics (see below).

    Globs provided here have precisely the same semantics as a single line in a gitignore file,
    where the meaning of `!` is inverted: namely, `!` at the beginning of a glob will ignore a file.
    Without `!`, all matches of the glob provided are treated as whitelist matches. [1]

    1. https://docs.rs/ignore/latest/ignore/overrides/struct.OverrideBuilder.html#method.add

# DURATION

For syntax of \<duration\>, see the documentation of `humantime::parse_duration`[1].

1. https://docs.rs/humantime/latest/humantime/fn.parse_duration.html

# SEE ALSO

**angrr**(1)

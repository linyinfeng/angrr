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

**owned_only** = \<bool\>
:   Only monitors owned symbolic link target of GC roots.

    If `angrr` is running as non-root user, the option will default to `true`, otherwise, default is `false`.

**remove_root** = \<bool\>
:   Remove GC root in **directory** instead of the symbolic link target of them.

    Default is `false`.

**directory** = [\<path1\>, \<path2\>, ...]
:   Directories containing auto GC roots.

    Default is `["/nix/var/nix/gcroots/auto"]`.

**temporary_root_policies** = [\<policy1\>, \<policy2\>, ...]
:   List of temporary root policies.

    See **TEMPORARY ROOT POLICY OPTIONS** for details.

**profile_root_policies** = [\<policy1\>, \<policy2\>, ...]
:   List of temporary root policies.

    See **PROFILE POLICY OPTIONS** for details.

# COMMMON POLICY OPTIONS

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

**path_regex** = \<regex\>
:   Only paths (absolute) matching the regex will be monitored by this policy.

**filter** = \<filter\>
:   An external program to filter paths that will be applied
    after all the other filter options.

    A JSON object containing the path information will be passed to the
    stdin of the program. If the program exits with code 0, then the
    path will be monitored; otherwise it will be ignored.

    SEE **FILTER OPTIONS** for more information.

**ignore_prefixes** = [\<prefix1\>, \<prefix2\>, ...]
:   A list of path prefixes (absolute) to ignore.

    Default is `["/nix/var/nix/profiles"]`.

**ignore_prefix_in_home** = [\<prefix1\>, \<prefix2\>, ...]
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

See **COMMON POLICY OPTIONS** for common options.

**profile_path** = \<path\>
:   Path to the profile.

**keep_since** = \<duration\>
:   Keep generations created within this duration.

**keep_latest_n** = \<number\>
:   Keep the latest \<number\> GC roots in this profile.

**keep_current_system** = \<bool\>
:   Whether to keep the current activated system generation.

    Only useful for system profiles. Default is `true`.

**keep_booted_system** = \<bool\>
:   Whether to keep the currently booted generation.

    Only useful for system profiles. Default is `true`.

# FILTER OPTIONS

**program** = \<path\>
:   Path to the external filter program.

**arguments** = [\<arg1\>, \<arg2\>, ...]
:   Arguments to pass to the external filter program.

# DURATION

For syntax of \<duration\>, see the documentation of `humantime::parse_duration`[1].

1. https://docs.rs/humantime/latest/humantime/fn.parse_duration.html

# SEE ALSO

**angrr**(1)

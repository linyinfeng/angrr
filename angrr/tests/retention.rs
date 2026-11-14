#![cfg(unix)]

mod support;

use std::{ffi::OsString, fs, path::PathBuf, time::Duration};

use regex::bytes::Regex;
use support::{
    DAY, Fixture, Location, assert_absent, assert_output_paths, assert_present, run, run_options,
    snapshot,
};

/// Expiry decides on age; a policy only sees roots matching its path regex and
/// not covered by its ignore prefixes.
#[test]
fn temporary_expiry_keeps_recent_ignored_and_unmatched_roots() {
    let fixture = Fixture::new();
    let expired_store = fixture.store_path("expired");
    let expired = fixture.root_link(Location::External, "projects/result old", &expired_store);
    expired.set_age(Duration::from_secs(14 * DAY));
    let expired_registration = fixture.register_auto(&expired);

    let recent = fixture.root_link(
        Location::External,
        "projects/result-recent",
        &fixture.store_path("recent"),
    );
    recent.set_age(Duration::from_secs(DAY));
    let recent_registration = fixture.register_auto(&recent);

    let ignored = fixture.root_link(
        Location::External,
        "projects/ignored/result",
        &fixture.store_path("ignored"),
    );
    ignored.set_age(Duration::from_secs(14 * DAY));
    let ignored_registration = fixture.register_auto(&ignored);

    let unmatched = fixture.root_link(
        Location::External,
        "projects/plain",
        &fixture.store_path("plain"),
    );
    unmatched.set_age(Duration::from_secs(14 * DAY));
    let unmatched_registration = fixture.register_auto(&unmatched);

    let mut config = fixture.config();
    let policy = config.temporary_root_policies.get_mut("results").unwrap();
    policy.period = Some(Duration::from_secs(7 * DAY));
    policy.path_regex = Regex::new("/result[^/]*$").unwrap();
    policy.ignore_prefixes = vec![
        fixture
            .directory(Location::External)
            .join("projects/ignored"),
    ];
    policy.ignore_prefixes_in_home.clear();
    let mut options = run_options(&["--no-prompt", "--no-statistic"]);
    options.output = Some(fixture.output.clone());
    let store_contents = fs::read(&expired_store.path).unwrap();

    run(options, config);

    assert_absent(&expired.path);
    // Removing the out-link leaves the registration behind as a dangling link.
    assert_present(&expired_registration);
    for (link, registration) in [
        (&recent, &recent_registration),
        (&ignored, &ignored_registration),
        (&unmatched, &unmatched_registration),
    ] {
        assert_present(&link.path);
        assert_present(registration);
    }
    assert_eq!(fs::read(&expired_store.path).unwrap(), store_contents);
    assert_output_paths(&fixture.output, b'\n', &[&expired.path]);
}

/// The default removal mode removes the out-link and keeps the auto registration.
#[test]
fn removing_the_out_link_keeps_the_auto_registration() {
    let fixture = Fixture::new();
    let store = fixture.store_path("expired");
    let out_link = fixture.root_link(Location::External, "projects/result-old", &store);
    out_link.set_age(Duration::from_secs(14 * DAY));
    let registration = fixture.register_auto(&out_link);

    let mut config = fixture.config();
    config.remove_root = false;
    config
        .temporary_root_policies
        .get_mut("results")
        .unwrap()
        .period = Some(Duration::from_secs(7 * DAY));
    let mut options = run_options(&["--no-prompt", "--no-statistic"]);
    options.output = Some(fixture.output.clone());

    run(options, config);

    assert_absent(&out_link.path);
    assert_present(&registration);
    assert_present(&store.path);
    assert_output_paths(&fixture.output, b'\n', &[&out_link.path]);
}

/// `remove-root` removes the registration in the scanned directory and leaves
/// the out-link intact.
#[test]
fn removing_the_auto_registration_keeps_the_out_link() {
    let fixture = Fixture::new();
    let store = fixture.store_path("expired");
    let out_link = fixture.root_link(Location::External, "projects/result-old", &store);
    out_link.set_age(Duration::from_secs(14 * DAY));
    let registration = fixture.register_auto(&out_link);

    let mut config = fixture.config();
    config.remove_root = true;
    config
        .temporary_root_policies
        .get_mut("results")
        .unwrap()
        .period = Some(Duration::from_secs(7 * DAY));
    let mut options = run_options(&["--no-prompt", "--no-statistic"]);
    options.output = Some(fixture.output.clone());

    run(options, config);

    assert_present(&out_link.path);
    assert_absent(&registration);
    assert_present(&store.path);
    assert_output_paths(&fixture.output, b'\n', &[&registration]);
}

/// Lower priority numbers are applied first. Both policies match the root, and
/// their names deliberately oppose their priorities: "z" (priority 1) keeps it
/// for 100 days, "a" (priority 2) would remove it after 7 days.
#[test]
fn temporary_policy_priority_outranks_a_lower_priority_policy() {
    let fixture = Fixture::new();
    let root = fixture.root_link(
        Location::External,
        "projects/result-priority",
        &fixture.store_path("priority"),
    );
    root.set_age(Duration::from_secs(14 * DAY));
    fixture.register_auto(&root);

    let mut config = fixture.config();
    let baseline = config.temporary_root_policies.remove("results").unwrap();
    let mut keep = baseline.clone();
    keep.priority = 1;
    keep.path_regex = Regex::new("/result-priority$").unwrap();
    keep.period = Some(Duration::from_secs(100 * DAY));
    let mut remove = baseline;
    remove.priority = 2;
    remove.path_regex = Regex::new(".*").unwrap();
    remove.period = Some(Duration::from_secs(7 * DAY));
    config.temporary_root_policies.insert("z".into(), keep);
    config.temporary_root_policies.insert("a".into(), remove);

    run(run_options(&["--no-prompt", "--no-statistic"]), config);

    assert_present(&root.path);
}

/// Equal priorities are ordered by policy name: "a" keeps for 100 days and "z"
/// would remove after 7 days, so the root survives only while names break ties.
#[test]
fn temporary_policy_name_breaks_a_priority_tie() {
    let fixture = Fixture::new();
    let root = fixture.root_link(
        Location::External,
        "projects/result-tie",
        &fixture.store_path("tie"),
    );
    root.set_age(Duration::from_secs(14 * DAY));
    fixture.register_auto(&root);

    let mut config = fixture.config();
    let baseline = config.temporary_root_policies.remove("results").unwrap();
    let mut keep = baseline.clone();
    keep.priority = 1;
    keep.path_regex = Regex::new(".*").unwrap();
    keep.period = Some(Duration::from_secs(100 * DAY));
    let mut remove = baseline;
    remove.priority = 1;
    remove.path_regex = Regex::new(".*").unwrap();
    remove.period = Some(Duration::from_secs(7 * DAY));
    config.temporary_root_policies.insert("a".into(), keep);
    config.temporary_root_policies.insert("z".into(), remove);

    run(run_options(&["--no-prompt", "--no-statistic"]), config);

    assert_present(&root.path);
}

/// The disabled policy would win on priority and remove the expired root, so
/// the root survives only while disabled policies are skipped.
#[test]
fn disabled_temporary_policies_are_skipped() {
    let fixture = Fixture::new();
    let root = fixture.root_link(
        Location::External,
        "projects/result-disabled",
        &fixture.store_path("disabled"),
    );
    root.set_age(Duration::from_secs(14 * DAY));
    fixture.register_auto(&root);

    let mut config = fixture.config();
    let mut enabled = config.temporary_root_policies.remove("results").unwrap();
    enabled.priority = 100;
    enabled.path_regex = Regex::new(".*").unwrap();
    enabled.period = Some(Duration::from_secs(100 * DAY));
    config
        .temporary_root_policies
        .insert("enabled".into(), enabled);
    // The baseline config ships this policy with `enable = false`.
    let disabled = config.temporary_root_policies.get_mut("disabled").unwrap();
    disabled.priority = 0;
    disabled.path_regex = Regex::new(".*").unwrap();
    disabled.period = Some(Duration::from_secs(7 * DAY));

    run(run_options(&["--no-prompt", "--no-statistic"]), config);

    assert_present(&root.path);
}

#[test]
fn profile_retention_keeps_current_latest_and_recent_generations() {
    let fixture = Fixture::new();
    let profile = fixture.profile(Location::Profiles, "profile");
    let expired = profile.generation(1, &fixture.store_path("gen-1"));
    let current = profile.generation(2, &fixture.store_path("gen-2"));
    let recent = profile.generation(3, &fixture.store_path("gen-3"));
    let also_expired = profile.generation(4, &fixture.store_path("gen-4"));
    let latest = profile.generation(10, &fixture.store_path("gen-10"));
    let unrelated = fixture.root_link(
        Location::Profiles,
        "other-99-link",
        &fixture.store_path("other"),
    );

    let mut registrations = Vec::new();
    for (link, age) in [
        (&expired, 14),
        (&current, 14),
        (&recent, 1),
        (&also_expired, 14),
        (&latest, 14),
        (&unrelated, 14),
    ] {
        link.set_age(Duration::from_secs(age * DAY));
        registrations.push(fixture.register_auto(link));
    }
    profile.set_current(2);

    let mut config = fixture.config();
    let policy = config.profile_policies.get_mut("user").unwrap();
    policy.profile_paths = vec![profile.path.clone()];
    policy.keep_since = Some(Duration::from_secs(7 * DAY));
    policy.keep_latest_n = Some(1);

    run(run_options(&["--no-prompt", "--no-statistic"]), config);

    // Kept: the current generation (2), the only recent one (3), and the latest
    // generation by number (10).
    for link in [&current, &recent, &latest] {
        assert_present(&link.path);
    }
    assert_absent(&expired.path);
    assert_absent(&also_expired.path);
    // The unrelated link shares the directory but does not belong to the profile.
    assert_present(&unrelated.path);
    for registration in &registrations {
        assert_present(registration);
    }
    assert_eq!(
        fs::read_link(&profile.path).unwrap(),
        PathBuf::from("profile-2-link")
    );
}

#[test]
fn dry_run_temporary_roots_report_the_removal_target_without_removing() {
    for remove_root in [false, true] {
        let fixture = Fixture::new();
        let expired = fixture.root_link(
            Location::External,
            "projects/result-old",
            &fixture.store_path("expired"),
        );
        expired.set_age(Duration::from_secs(14 * DAY));
        let registration = fixture.register_auto(&expired);

        let mut config = fixture.config();
        config.remove_root = remove_root;
        config
            .temporary_root_policies
            .get_mut("results")
            .unwrap()
            .period = Some(Duration::from_secs(7 * DAY));
        let expected = if remove_root {
            &registration
        } else {
            &expired.path
        };
        let mut options = run_options(&["--no-prompt", "--no-statistic"]);
        options.dry_run = true;
        options.output = Some(fixture.output.clone());
        let before = snapshot(&fixture.path);

        run(options, config);

        assert_output_paths(&fixture.output, b'\n', &[expected]);
        assert_eq!(snapshot(&fixture.path), before);
    }
}

#[test]
fn dry_run_profile_generations_report_without_removing() {
    let fixture = Fixture::new();
    let profile = fixture.profile(Location::Profiles, "profile");
    let expired = profile.generation(1, &fixture.store_path("gen-1"));
    let latest = profile.generation(2, &fixture.store_path("gen-2"));
    for link in [&expired, &latest] {
        link.set_age(Duration::from_secs(14 * DAY));
        fixture.register_auto(link);
    }
    profile.set_current(2);

    let mut config = fixture.config();
    let policy = config.profile_policies.get_mut("user").unwrap();
    policy.profile_paths = vec![profile.path.clone()];
    policy.keep_latest_n = Some(1);
    let mut options = run_options(&["--no-prompt", "--no-statistic"]);
    options.dry_run = true;
    options.output = Some(fixture.output.clone());
    let before = snapshot(&fixture.path);

    run(options, config);

    assert_output_paths(&fixture.output, b'\n', &[&expired.path]);
    assert_eq!(snapshot(&fixture.path), before);
}

#[test]
fn output_delimiter_separates_paths_without_a_trailing_delimiter() {
    for delimiter in ["\n", "\0", "|"] {
        let fixture = Fixture::new();
        let first = fixture.root_link(
            Location::External,
            "projects/result-a",
            &fixture.store_path("a"),
        );
        let second = fixture.root_link(
            Location::External,
            "projects/result-b",
            &fixture.store_path("b"),
        );
        for link in [&first, &second] {
            link.set_age(Duration::from_secs(14 * DAY));
            fixture.register_auto(link);
        }

        let mut config = fixture.config();
        config
            .temporary_root_policies
            .get_mut("results")
            .unwrap()
            .period = Some(Duration::from_secs(7 * DAY));
        let mut options = run_options(&["--no-prompt", "--no-statistic"]);
        options.output = Some(fixture.output.clone());
        options.output_delimiter = OsString::from(delimiter);

        run(options, config);

        let output = fs::read(&fixture.output).unwrap();
        assert!(!output.ends_with(delimiter.as_bytes()));
        assert_output_paths(
            &fixture.output,
            delimiter.as_bytes()[0],
            &[&first.path, &second.path],
        );
        assert_absent(&first.path);
        assert_absent(&second.path);
    }
}

#[test]
fn unbuffered_output_records_the_removed_path() {
    let fixture = Fixture::new();
    let root = fixture.root_link(
        Location::External,
        "projects/result-unbuffered",
        &fixture.store_path("unbuffered"),
    );
    root.set_age(Duration::from_secs(14 * DAY));
    fixture.register_auto(&root);

    let mut config = fixture.config();
    config
        .temporary_root_policies
        .get_mut("results")
        .unwrap()
        .period = Some(Duration::from_secs(7 * DAY));
    let mut options = run_options(&["--no-prompt", "--no-statistic"]);
    options.output = Some(fixture.output.clone());
    options.output_unbuffered = true;

    run(options, config);

    assert_output_paths(&fixture.output, b'\n', &[&root.path]);
    assert_absent(&root.path);
}

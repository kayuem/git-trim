mod fixture;

use std::convert::TryFrom;

use anyhow::Result;
use git2::Repository;

use git_trim::{get_merged_or_gone, Config, Git, MergedOrGone, RemoteBranch};

use fixture::{rc, Fixture};
use git_trim::args::DeleteFilter;

fn fixture() -> Fixture {
    rc().append_fixture_trace(
        r#"
        git init upstream
        upstream <<EOF
            git config user.name "UpstreamTest"
            git config user.email "upstream@test"
            echo "Hello World!" > README.md
            git add README.md
            git commit -m "Initial commit"
        EOF
        git clone upstream origin -o upstream
        origin <<EOF
            git config user.name "Origin Test"
            git config user.email "origin@test"
            git config remote.pushdefault upstream
        EOF
        git clone origin local
        local <<EOF
            git config user.name "Local Test"
            git config user.email "local@test"
            git config remote.pushdefault origin
            git config push.default simple
            git remote add upstream ../upstream
            git fetch upstream
            git branch -u upstream/master master
        EOF
        origin <<EOF
            git checkout -b feature
            touch awesome-patch
            git add awesome-patch
            git commit -m "Awesome patch"
            git push upstream feature:refs/pull/1/head
            git checkout master
        EOF
        "#,
    )
}

fn config() -> Config<'static> {
    Config {
        bases: vec!["master"],
        protected_branches: set! {},
        filter: DeleteFilter::all(),
        detach: true,
    }
}

#[test]
fn test_accepted() -> Result<()> {
    let guard = fixture().prepare(
        "local",
        r#"
        local <<EOF
            git fetch ../origin feature:feature
            git config branch.feature.remote "../origin"
            git config branch.feature.merge "refs/heads/feature"
        EOF
        upstream <<EOF
            git merge refs/pull/1/head
        EOF
        # clicked delete branch button
        origin <<EOF
            git branch -D feature
        EOF
        "#,
    )?;

    let git = Git::try_from(Repository::open(guard.working_directory())?)?;
    let branches = get_merged_or_gone(&git, &config())?;
    assert_eq!(
        branches.to_delete,
        MergedOrGone {
            merged_locals: set! {"feature"},
            ..Default::default()
        },
    );
    Ok(())
}

#[test]
fn test_accepted_but_forgot_to_delete() -> Result<()> {
    let guard = fixture().prepare(
        "local",
        r#"
        local <<EOF
            git fetch ../origin feature:feature
            git config branch.feature.remote "../origin"
            git config branch.feature.merge "refs/heads/feature"
        EOF
        upstream <<EOF
            git merge refs/pull/1/head
        EOF
        "#,
    )?;

    let git = Git::try_from(Repository::open(guard.working_directory())?)?;
    let branches = get_merged_or_gone(&git, &config())?;
    assert_eq!(
        branches.to_delete,
        MergedOrGone {
            merged_locals: set! {"feature"},
            merged_remotes: set! {
                RemoteBranch {
                    remote: "../origin".to_string(),
                    refname: "refs/heads/feature".to_string(),
                },
            },
            ..Default::default()
        },
    );
    Ok(())
}

#[test]
fn test_should_not_push_delete_non_heads() -> Result<()> {
    let guard = fixture().prepare(
        "local",
        r#"
        local <<EOF
            git fetch ../upstream refs/pull/1/head:feature
            git config branch.feature.remote "../origin"
            git config branch.feature.merge "refs/pull/1/head"
        EOF
        upstream <<EOF
            git merge refs/pull/1/head
        EOF
        "#,
    )?;

    let git = Git::try_from(Repository::open(guard.working_directory())?)?;
    let branches = get_merged_or_gone(&git, &config())?;
    assert_eq!(
        branches.to_delete,
        MergedOrGone {
            merged_locals: set! {"feature"},
            ..Default::default()
        },
    );
    Ok(())
}

use builtin;
use str;

set edit:completion:arg-completer[vissue] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'vissue'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'vissue'= {
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
            cand create 'Create an issue. Pass the body with --body or --body-file (`-` reads stdin); omit both to leave the body empty for a later edit'
            cand q 'Quick capture: create and print only the id'
            cand list 'List issues, sorted by priority then state then id'
            cand show 'Show one issue: metadata, then the body'
            cand update 'Update state, priority, or blocker edges'
            cand resolve 'Pick one terminal after a sibling close'
            cand reject 'Reject an issue, redirecting to an existing destination or a new replacement'
            cand ready 'Actionable issues: TODO or STARTED with no open blocker'
            cand claim 'Take an issue: move it to STARTED and stamp the claim'
            cand note 'Add a dated note to the top of an issue''s logbook; state and claim untouched'
            cand append 'Append a dated report to an issue''s body'
            cand claims 'Every live claim, oldest first: who holds what, and for how long'
            cand fold 'Fold an inbox org file: each unstamped `* TODO <title>` heading becomes an issue, then the heading is stamped with the id and flipped to DONE in place. Already-stamped headings are skipped'
            cand agenda 'Dated open work: deadlines and scheduled starts inside a horizon, overdue first'
            cand hygiene 'Checklist for agents and CI: stalled claims plus corpus validation'
            cand whoami 'Print the identity this tracker would record on a claim'
            cand waiting-on 'Issues waiting on this one'
            cand body-excerpt 'The first lines of an issue''s file range'
            cand search 'Substring search over ids, titles, properties, and bodies'
            cand children 'Issues whose `:PARENT:` matches this id'
            cand ancestors 'Blockers transitively required by this issue'
            cand impact 'Issues transitively waiting on this issue'
            cand related 'Explain bounded Org and lexical connections around an issue'
            cand stale 'Open issues whose `:CREATED:` is older than N days'
            cand count 'Print only the matching issue count'
            cand export 'One JSON object per issue per line'
            cand tree 'Children and blockers below an id'
            cand cycles 'Cycles in the blocker graph'
            cand graph 'The blocker and parent graph as Graphviz DOT'
            cand refile 'Move an issue to another project''s file'
            cand backlinks 'Issues referring to this id'
            cand roadmap 'A markdown roadmap of active and closed work'
            cand check 'Validate the corpus. Exits non-zero on any error'
            cand digest 'A content digest of the corpus, for telling whether a copy is current'
            cand mirror 'Write a read-only projection of one or more projects to a file'
            cand events 'Change events with a sequence above --since'
            cand ping 'Append a manual event, waking pollers without editing an issue'
            cand wait 'Block until the generation passes --last, or until an issue is terminal. Exits 2 on timeout'
            cand gen 'Print the current generation counter'
            cand projects 'List the projects found under the layout prefix'
            cand identity 'Print the resolved binary, root, and prefix'
            cand serve 'Own the per-user Unix control socket'
            cand tui 'Interactive board over ready, list, claims, agenda, and search'
            cand hud 'Task board. Default execs `vissue-hud` (Ready / Mine / Upcoming / All)'
            cand completions 'Write a shell completion script to stdout'
            cand man 'Write the roff manual page to stdout'
            cand keys 'Print the HUD key catalog, or check a keys.toml overlay'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'vissue;create'= {
            cand -p 'Project name. Auto-detected from .project-ctx.toml when omitted'
            cand --project 'Project name. Auto-detected from .project-ctx.toml when omitted'
            cand --priority 'Priority cookie: A high, B mid, C low'
            cand -t 'Type tag such as feature, bug, or task'
            cand --type 'Type tag such as feature, bug, or task'
            cand --deadline 'Org deadline like `<2026-05-15 Fri>` or `[2026-05-15]`'
            cand --scheduled 'Org scheduled date like `<2026-05-01 Mon>`'
            cand --tags 'Comma- or colon-separated tags'
            cand --parent 'Parent id, which must already exist'
            cand --body 'Body text written under the heading'
            cand --body-file 'Read the body from a file; `-` reads stdin'
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand -q 'Print only the new id'
            cand --quiet 'Print only the new id'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;q'= {
            cand -p 'p'
            cand --project 'project'
            cand -t 't'
            cand --type 'type'
            cand --parent 'parent'
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;list'= {
            cand -p 'p'
            cand --project 'project'
            cand -s 'Filter by state: TODO, STARTED, BLOCKED, DONE, or CANCELLED'
            cand --state 'Filter by state: TODO, STARTED, BLOCKED, DONE, or CANCELLED'
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --json 'Emit JSON rows instead of text'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;show'= {
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --json 'Emit a JSON object instead of text'
            cand --org 'Emit the heading''s org text in full, nothing else. Use this to write the issue out as the specification someone works from'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;update'= {
            cand -s 's'
            cand --state 'state'
            cand --priority 'priority'
            cand --block 'Add a blocker edge'
            cand --unblock 'Remove a blocker edge'
            cand --if-state 'Refuse unless the heading is still this state'
            cand --if-gen 'Refuse unless the corpus generation is still this value'
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;resolve'= {
            cand -s 's'
            cand --state 'state'
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;reject'= {
            cand --to 'Existing destination issue'
            cand -p 'Project for a newly created replacement'
            cand --project 'Project for a newly created replacement'
            cand --reason 'Why this issue is rejected'
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;ready'= {
            cand -p 'p'
            cand --project 'project'
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --json 'json'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;claim'= {
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --force 'Take over a claim held by another identity'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;note'= {
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;append'= {
            cand --text 'The text to append'
            cand --file 'Read the text from a file; `-` reads stdin'
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'vissue;claims'= {
            cand --by 'Only claims held by this identity'
            cand -p 'Only claims in this project'
            cand --project 'Only claims in this project'
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --json 'Machine-readable output'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;fold'= {
            cand -p 'Project the folded issues are created in. Auto-detected from .project-ctx.toml when omitted'
            cand --project 'Project the folded issues are created in. Auto-detected from .project-ctx.toml when omitted'
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;agenda'= {
            cand -d 'Days ahead to include'
            cand --days 'Days ahead to include'
            cand -p 'p'
            cand --project 'project'
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;hygiene'= {
            cand --stale-days 'Days a claim may be held before it counts as stale'
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;whoami'= {
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;waiting-on'= {
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;body-excerpt'= {
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;search'= {
            cand -n 'n'
            cand --limit 'limit'
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;children'= {
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;ancestors'= {
            cand -d 'd'
            cand --depth 'depth'
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;impact'= {
            cand -d 'd'
            cand --depth 'depth'
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;related'= {
            cand -d 'd'
            cand --depth 'depth'
            cand -n 'n'
            cand --limit 'limit'
            cand --format 'text or org; org emits links to the source headings'
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;stale'= {
            cand -d 'd'
            cand --days 'days'
            cand -p 'p'
            cand --project 'project'
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;count'= {
            cand -p 'p'
            cand --project 'project'
            cand -s 's'
            cand --state 'state'
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand -r 'Count only actionable issues'
            cand --ready 'Count only actionable issues'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;export'= {
            cand -p 'p'
            cand --project 'project'
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;tree'= {
            cand -f 'ascii or dot'
            cand --format 'ascii or dot'
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;cycles'= {
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;graph'= {
            cand -p 'p'
            cand --project 'project'
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;refile'= {
            cand --to 'Target project'
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;backlinks'= {
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;roadmap'= {
            cand -p 'p'
            cand --project 'project'
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;check'= {
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;digest'= {
            cand -p 'Project to include; repeat for several. Omit for every project'
            cand --project 'Project to include; repeat for several. Omit for every project'
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --json 'Emit a JSON object instead of text'
            cand -q 'Print only the combined digest'
            cand --quiet 'Print only the combined digest'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;mirror'= {
            cand -p 'Project to include; repeat for several. Omit for every project'
            cand --project 'Project to include; repeat for several. Omit for every project'
            cand -o 'Destination file; `-` writes to standard output'
            cand --out 'Destination file; `-` writes to standard output'
            cand --check 'Compare an existing mirror''s stamp against the tracker instead of writing. Exits 0 when fresh, 1 when stale'
            cand -f 'org or markdown'
            cand --format 'org or markdown'
            cand -s 'Include only this state'
            cand --state 'Include only this state'
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;events'= {
            cand --since 'Only events newer than this sequence'
            cand -n 'Maximum events returned'
            cand --limit 'Maximum events returned'
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;ping'= {
            cand --detail 'detail'
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;wait'= {
            cand --last 'last'
            cand --id 'Issue to watch when --until-terminal is set'
            cand --poll-ms 'poll-ms'
            cand --timeout-ms 'timeout-ms'
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --until-terminal 'Block until the issue is DONE or CANCELLED'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;gen'= {
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;projects'= {
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;identity'= {
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;serve'= {
            cand -s 'Control socket path. Falls back to VISSUE_CONTROL_SOCKET, then $XDG_RUNTIME_DIR/vissue/control.sock, then ~/.vissue/run/control.sock'
            cand --socket 'Control socket path. Falls back to VISSUE_CONTROL_SOCKET, then $XDG_RUNTIME_DIR/vissue/control.sock, then ~/.vissue/run/control.sock'
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand -d 'Detach after the socket accepts. The child is placed in its own process group (not a new session) and can still receive SIGHUP from the parent terminal'
            cand --detach 'Detach after the socket accepts. The child is placed in its own process group (not a new session) and can still receive SIGHUP from the parent terminal'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
            cand stop 'Signal the owner (SIGTERM, then SIGKILL) and wait'
            cand restart 'Stop, then start detached'
            cand status 'Print a live/pid/socket snapshot. Exit 0 if live, 1 otherwise'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'vissue;serve;stop'= {
            cand -s 'Control socket path. Falls back to VISSUE_CONTROL_SOCKET, then $XDG_RUNTIME_DIR/vissue/control.sock, then ~/.vissue/run/control.sock'
            cand --socket 'Control socket path. Falls back to VISSUE_CONTROL_SOCKET, then $XDG_RUNTIME_DIR/vissue/control.sock, then ~/.vissue/run/control.sock'
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;serve;restart'= {
            cand -s 'Control socket path. Falls back to VISSUE_CONTROL_SOCKET, then $XDG_RUNTIME_DIR/vissue/control.sock, then ~/.vissue/run/control.sock'
            cand --socket 'Control socket path. Falls back to VISSUE_CONTROL_SOCKET, then $XDG_RUNTIME_DIR/vissue/control.sock, then ~/.vissue/run/control.sock'
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;serve;status'= {
            cand -s 'Control socket path. Falls back to VISSUE_CONTROL_SOCKET, then $XDG_RUNTIME_DIR/vissue/control.sock, then ~/.vissue/run/control.sock'
            cand --socket 'Control socket path. Falls back to VISSUE_CONTROL_SOCKET, then $XDG_RUNTIME_DIR/vissue/control.sock, then ~/.vissue/run/control.sock'
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --json 'Machine-readable object'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;serve;help'= {
            cand stop 'Signal the owner (SIGTERM, then SIGKILL) and wait'
            cand restart 'Stop, then start detached'
            cand status 'Print a live/pid/socket snapshot. Exit 0 if live, 1 otherwise'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'vissue;serve;help;stop'= {
        }
        &'vissue;serve;help;restart'= {
        }
        &'vissue;serve;help;status'= {
        }
        &'vissue;serve;help;help'= {
        }
        &'vissue;tui'= {
            cand -s 'Control socket path. Falls back to VISSUE_CONTROL_SOCKET, then $XDG_RUNTIME_DIR/vissue/control.sock, then ~/.vissue/run/control.sock'
            cand --socket 'Control socket path. Falls back to VISSUE_CONTROL_SOCKET, then $XDG_RUNTIME_DIR/vissue/control.sock, then ~/.vissue/run/control.sock'
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --offline 'Never attach, never spawn serve; CatalogService plus generation poll'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'vissue;hud'= {
            cand --mode 'ready, list (all), claims, stale, or new. Used by `--rofi`'
            cand -s 'Control socket path. Falls back to VISSUE_CONTROL_SOCKET, then $XDG_RUNTIME_DIR/vissue/control.sock, then ~/.vissue/run/control.sock'
            cand --socket 'Control socket path. Falls back to VISSUE_CONTROL_SOCKET, then $XDG_RUNTIME_DIR/vissue/control.sock, then ~/.vissue/run/control.sock'
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --offline 'Never attach, never spawn serve'
            cand --toggle 'Show or hide a running board, or dismiss a live rofi picker'
            cand --show 'Show a running board'
            cand --hide 'Hide a running board, or dismiss a live rofi picker'
            cand --iced 'Use the iced board. Default when `--rofi` is absent'
            cand --rofi 'Use the rofi picker instead of the iced board'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'vissue;completions'= {
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'vissue;man'= {
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'vissue;keys'= {
            cand --root 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory'
            cand --prefix 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`'
            cand --check 'Load the overlay and exit 1 on conflict'
            cand --occupancy 'Print taken chords'
            cand --no-route 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
        }
        &'vissue;help'= {
            cand create 'Create an issue. Pass the body with --body or --body-file (`-` reads stdin); omit both to leave the body empty for a later edit'
            cand q 'Quick capture: create and print only the id'
            cand list 'List issues, sorted by priority then state then id'
            cand show 'Show one issue: metadata, then the body'
            cand update 'Update state, priority, or blocker edges'
            cand resolve 'Pick one terminal after a sibling close'
            cand reject 'Reject an issue, redirecting to an existing destination or a new replacement'
            cand ready 'Actionable issues: TODO or STARTED with no open blocker'
            cand claim 'Take an issue: move it to STARTED and stamp the claim'
            cand note 'Add a dated note to the top of an issue''s logbook; state and claim untouched'
            cand append 'Append a dated report to an issue''s body'
            cand claims 'Every live claim, oldest first: who holds what, and for how long'
            cand fold 'Fold an inbox org file: each unstamped `* TODO <title>` heading becomes an issue, then the heading is stamped with the id and flipped to DONE in place. Already-stamped headings are skipped'
            cand agenda 'Dated open work: deadlines and scheduled starts inside a horizon, overdue first'
            cand hygiene 'Checklist for agents and CI: stalled claims plus corpus validation'
            cand whoami 'Print the identity this tracker would record on a claim'
            cand waiting-on 'Issues waiting on this one'
            cand body-excerpt 'The first lines of an issue''s file range'
            cand search 'Substring search over ids, titles, properties, and bodies'
            cand children 'Issues whose `:PARENT:` matches this id'
            cand ancestors 'Blockers transitively required by this issue'
            cand impact 'Issues transitively waiting on this issue'
            cand related 'Explain bounded Org and lexical connections around an issue'
            cand stale 'Open issues whose `:CREATED:` is older than N days'
            cand count 'Print only the matching issue count'
            cand export 'One JSON object per issue per line'
            cand tree 'Children and blockers below an id'
            cand cycles 'Cycles in the blocker graph'
            cand graph 'The blocker and parent graph as Graphviz DOT'
            cand refile 'Move an issue to another project''s file'
            cand backlinks 'Issues referring to this id'
            cand roadmap 'A markdown roadmap of active and closed work'
            cand check 'Validate the corpus. Exits non-zero on any error'
            cand digest 'A content digest of the corpus, for telling whether a copy is current'
            cand mirror 'Write a read-only projection of one or more projects to a file'
            cand events 'Change events with a sequence above --since'
            cand ping 'Append a manual event, waking pollers without editing an issue'
            cand wait 'Block until the generation passes --last, or until an issue is terminal. Exits 2 on timeout'
            cand gen 'Print the current generation counter'
            cand projects 'List the projects found under the layout prefix'
            cand identity 'Print the resolved binary, root, and prefix'
            cand serve 'Own the per-user Unix control socket'
            cand tui 'Interactive board over ready, list, claims, agenda, and search'
            cand hud 'Task board. Default execs `vissue-hud` (Ready / Mine / Upcoming / All)'
            cand completions 'Write a shell completion script to stdout'
            cand man 'Write the roff manual page to stdout'
            cand keys 'Print the HUD key catalog, or check a keys.toml overlay'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'vissue;help;create'= {
        }
        &'vissue;help;q'= {
        }
        &'vissue;help;list'= {
        }
        &'vissue;help;show'= {
        }
        &'vissue;help;update'= {
        }
        &'vissue;help;resolve'= {
        }
        &'vissue;help;reject'= {
        }
        &'vissue;help;ready'= {
        }
        &'vissue;help;claim'= {
        }
        &'vissue;help;note'= {
        }
        &'vissue;help;append'= {
        }
        &'vissue;help;claims'= {
        }
        &'vissue;help;fold'= {
        }
        &'vissue;help;agenda'= {
        }
        &'vissue;help;hygiene'= {
        }
        &'vissue;help;whoami'= {
        }
        &'vissue;help;waiting-on'= {
        }
        &'vissue;help;body-excerpt'= {
        }
        &'vissue;help;search'= {
        }
        &'vissue;help;children'= {
        }
        &'vissue;help;ancestors'= {
        }
        &'vissue;help;impact'= {
        }
        &'vissue;help;related'= {
        }
        &'vissue;help;stale'= {
        }
        &'vissue;help;count'= {
        }
        &'vissue;help;export'= {
        }
        &'vissue;help;tree'= {
        }
        &'vissue;help;cycles'= {
        }
        &'vissue;help;graph'= {
        }
        &'vissue;help;refile'= {
        }
        &'vissue;help;backlinks'= {
        }
        &'vissue;help;roadmap'= {
        }
        &'vissue;help;check'= {
        }
        &'vissue;help;digest'= {
        }
        &'vissue;help;mirror'= {
        }
        &'vissue;help;events'= {
        }
        &'vissue;help;ping'= {
        }
        &'vissue;help;wait'= {
        }
        &'vissue;help;gen'= {
        }
        &'vissue;help;projects'= {
        }
        &'vissue;help;identity'= {
        }
        &'vissue;help;serve'= {
            cand stop 'Signal the owner (SIGTERM, then SIGKILL) and wait'
            cand restart 'Stop, then start detached'
            cand status 'Print a live/pid/socket snapshot. Exit 0 if live, 1 otherwise'
        }
        &'vissue;help;serve;stop'= {
        }
        &'vissue;help;serve;restart'= {
        }
        &'vissue;help;serve;status'= {
        }
        &'vissue;help;tui'= {
        }
        &'vissue;help;hud'= {
        }
        &'vissue;help;completions'= {
        }
        &'vissue;help;man'= {
        }
        &'vissue;help;keys'= {
        }
        &'vissue;help;help'= {
        }
    ]
    $completions[$command]
}

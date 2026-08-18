
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'vissue' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'vissue'
        for ($i = 1; $i -lt $commandElements.Count; $i++) {
            $element = $commandElements[$i]
            if ($element -isnot [StringConstantExpressionAst] -or
                $element.StringConstantType -ne [StringConstantType]::BareWord -or
                $element.Value.StartsWith('-') -or
                $element.Value -eq $wordToComplete) {
                break
        }
        $element.Value
    }) -join ';'

    $completions = @(switch ($command) {
        'vissue' {
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Create an issue. Pass the body with --body or --body-file (`-` reads stdin); omit both to leave the body empty for a later edit')
            [CompletionResult]::new('q', 'q', [CompletionResultType]::ParameterValue, 'Quick capture: create and print only the id')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List issues, sorted by priority then state then id')
            [CompletionResult]::new('show', 'show', [CompletionResultType]::ParameterValue, 'Show one issue: metadata, then the body')
            [CompletionResult]::new('update', 'update', [CompletionResultType]::ParameterValue, 'Update state, priority, or blocker edges')
            [CompletionResult]::new('resolve', 'resolve', [CompletionResultType]::ParameterValue, 'Pick one terminal after a sibling close')
            [CompletionResult]::new('reject', 'reject', [CompletionResultType]::ParameterValue, 'Reject an issue, redirecting to an existing destination or a new replacement')
            [CompletionResult]::new('ready', 'ready', [CompletionResultType]::ParameterValue, 'Actionable issues: TODO or STARTED with no open blocker')
            [CompletionResult]::new('claim', 'claim', [CompletionResultType]::ParameterValue, 'Take an issue: move it to STARTED and stamp the claim')
            [CompletionResult]::new('note', 'note', [CompletionResultType]::ParameterValue, 'Add a dated note to the top of an issue''s logbook; state and claim untouched')
            [CompletionResult]::new('append', 'append', [CompletionResultType]::ParameterValue, 'Append a dated report to an issue''s body')
            [CompletionResult]::new('claims', 'claims', [CompletionResultType]::ParameterValue, 'Every live claim, oldest first: who holds what, and for how long')
            [CompletionResult]::new('fold', 'fold', [CompletionResultType]::ParameterValue, 'Fold an inbox org file: each unstamped `* TODO <title>` heading becomes an issue, then the heading is stamped with the id and flipped to DONE in place. Already-stamped headings are skipped')
            [CompletionResult]::new('agenda', 'agenda', [CompletionResultType]::ParameterValue, 'Dated open work: deadlines and scheduled starts inside a horizon, overdue first')
            [CompletionResult]::new('hygiene', 'hygiene', [CompletionResultType]::ParameterValue, 'Checklist for agents and CI: stalled claims plus corpus validation')
            [CompletionResult]::new('whoami', 'whoami', [CompletionResultType]::ParameterValue, 'Print the identity this tracker would record on a claim')
            [CompletionResult]::new('waiting-on', 'waiting-on', [CompletionResultType]::ParameterValue, 'Issues waiting on this one')
            [CompletionResult]::new('body-excerpt', 'body-excerpt', [CompletionResultType]::ParameterValue, 'The first lines of an issue''s file range')
            [CompletionResult]::new('search', 'search', [CompletionResultType]::ParameterValue, 'Substring search over ids, titles, properties, and bodies')
            [CompletionResult]::new('children', 'children', [CompletionResultType]::ParameterValue, 'Issues whose `:PARENT:` matches this id')
            [CompletionResult]::new('ancestors', 'ancestors', [CompletionResultType]::ParameterValue, 'Blockers transitively required by this issue')
            [CompletionResult]::new('impact', 'impact', [CompletionResultType]::ParameterValue, 'Issues transitively waiting on this issue')
            [CompletionResult]::new('related', 'related', [CompletionResultType]::ParameterValue, 'Explain bounded Org and lexical connections around an issue')
            [CompletionResult]::new('stale', 'stale', [CompletionResultType]::ParameterValue, 'Open issues whose `:CREATED:` is older than N days')
            [CompletionResult]::new('count', 'count', [CompletionResultType]::ParameterValue, 'Print only the matching issue count')
            [CompletionResult]::new('export', 'export', [CompletionResultType]::ParameterValue, 'One JSON object per issue per line')
            [CompletionResult]::new('tree', 'tree', [CompletionResultType]::ParameterValue, 'Children and blockers below an id')
            [CompletionResult]::new('cycles', 'cycles', [CompletionResultType]::ParameterValue, 'Cycles in the blocker graph')
            [CompletionResult]::new('graph', 'graph', [CompletionResultType]::ParameterValue, 'The blocker and parent graph as Graphviz DOT')
            [CompletionResult]::new('refile', 'refile', [CompletionResultType]::ParameterValue, 'Move an issue to another project''s file')
            [CompletionResult]::new('backlinks', 'backlinks', [CompletionResultType]::ParameterValue, 'Issues referring to this id')
            [CompletionResult]::new('roadmap', 'roadmap', [CompletionResultType]::ParameterValue, 'A markdown roadmap of active and closed work')
            [CompletionResult]::new('check', 'check', [CompletionResultType]::ParameterValue, 'Validate the corpus. Exits non-zero on any error')
            [CompletionResult]::new('digest', 'digest', [CompletionResultType]::ParameterValue, 'A content digest of the corpus, for telling whether a copy is current')
            [CompletionResult]::new('mirror', 'mirror', [CompletionResultType]::ParameterValue, 'Write a read-only projection of one or more projects to a file')
            [CompletionResult]::new('events', 'events', [CompletionResultType]::ParameterValue, 'Change events with a sequence above --since')
            [CompletionResult]::new('ping', 'ping', [CompletionResultType]::ParameterValue, 'Append a manual event, waking pollers without editing an issue')
            [CompletionResult]::new('wait', 'wait', [CompletionResultType]::ParameterValue, 'Block until the generation passes --last, or until an issue is terminal. Exits 2 on timeout')
            [CompletionResult]::new('gen', 'gen', [CompletionResultType]::ParameterValue, 'Print the current generation counter')
            [CompletionResult]::new('projects', 'projects', [CompletionResultType]::ParameterValue, 'List the projects found under the layout prefix')
            [CompletionResult]::new('identity', 'identity', [CompletionResultType]::ParameterValue, 'Print the resolved binary, root, and prefix')
            [CompletionResult]::new('serve', 'serve', [CompletionResultType]::ParameterValue, 'Own the per-user Unix control socket')
            [CompletionResult]::new('tui', 'tui', [CompletionResultType]::ParameterValue, 'Interactive board over ready, list, claims, agenda, and search')
            [CompletionResult]::new('hud', 'hud', [CompletionResultType]::ParameterValue, 'Task board. Default execs `vissue-hud` (Ready / Mine / Upcoming / All)')
            [CompletionResult]::new('completions', 'completions', [CompletionResultType]::ParameterValue, 'Write a shell completion script to stdout')
            [CompletionResult]::new('man', 'man', [CompletionResultType]::ParameterValue, 'Write the roff manual page to stdout')
            [CompletionResult]::new('keys', 'keys', [CompletionResultType]::ParameterValue, 'Print the HUD key catalog, or check a keys.toml overlay')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'vissue;create' {
            [CompletionResult]::new('-p', '-p', [CompletionResultType]::ParameterName, 'Project name. Auto-detected from .project-ctx.toml when omitted')
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'Project name. Auto-detected from .project-ctx.toml when omitted')
            [CompletionResult]::new('--priority', '--priority', [CompletionResultType]::ParameterName, 'Priority cookie: A high, B mid, C low')
            [CompletionResult]::new('-t', '-t', [CompletionResultType]::ParameterName, 'Type tag such as feature, bug, or task')
            [CompletionResult]::new('--type', '--type', [CompletionResultType]::ParameterName, 'Type tag such as feature, bug, or task')
            [CompletionResult]::new('--deadline', '--deadline', [CompletionResultType]::ParameterName, 'Org deadline like `<2026-05-15 Fri>` or `[2026-05-15]`')
            [CompletionResult]::new('--scheduled', '--scheduled', [CompletionResultType]::ParameterName, 'Org scheduled date like `<2026-05-01 Mon>`')
            [CompletionResult]::new('--tags', '--tags', [CompletionResultType]::ParameterName, 'Comma- or colon-separated tags')
            [CompletionResult]::new('--parent', '--parent', [CompletionResultType]::ParameterName, 'Parent id, which must already exist')
            [CompletionResult]::new('--body', '--body', [CompletionResultType]::ParameterName, 'Body text written under the heading')
            [CompletionResult]::new('--body-file', '--body-file', [CompletionResultType]::ParameterName, 'Read the body from a file; `-` reads stdin')
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Print only the new id')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Print only the new id')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;q' {
            [CompletionResult]::new('-p', '-p', [CompletionResultType]::ParameterName, 'p')
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'project')
            [CompletionResult]::new('-t', '-t', [CompletionResultType]::ParameterName, 't')
            [CompletionResult]::new('--type', '--type', [CompletionResultType]::ParameterName, 'type')
            [CompletionResult]::new('--parent', '--parent', [CompletionResultType]::ParameterName, 'parent')
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;list' {
            [CompletionResult]::new('-p', '-p', [CompletionResultType]::ParameterName, 'p')
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'project')
            [CompletionResult]::new('-s', '-s', [CompletionResultType]::ParameterName, 'Filter by state: TODO, STARTED, BLOCKED, DONE, or CANCELLED')
            [CompletionResult]::new('--state', '--state', [CompletionResultType]::ParameterName, 'Filter by state: TODO, STARTED, BLOCKED, DONE, or CANCELLED')
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit JSON rows instead of text')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;show' {
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit a JSON object instead of text')
            [CompletionResult]::new('--org', '--org', [CompletionResultType]::ParameterName, 'Emit the heading''s org text in full, nothing else. Use this to write the issue out as the specification someone works from')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;update' {
            [CompletionResult]::new('-s', '-s', [CompletionResultType]::ParameterName, 's')
            [CompletionResult]::new('--state', '--state', [CompletionResultType]::ParameterName, 'state')
            [CompletionResult]::new('--priority', '--priority', [CompletionResultType]::ParameterName, 'priority')
            [CompletionResult]::new('--block', '--block', [CompletionResultType]::ParameterName, 'Add a blocker edge')
            [CompletionResult]::new('--unblock', '--unblock', [CompletionResultType]::ParameterName, 'Remove a blocker edge')
            [CompletionResult]::new('--if-state', '--if-state', [CompletionResultType]::ParameterName, 'Refuse unless the heading is still this state')
            [CompletionResult]::new('--if-gen', '--if-gen', [CompletionResultType]::ParameterName, 'Refuse unless the corpus generation is still this value')
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;resolve' {
            [CompletionResult]::new('-s', '-s', [CompletionResultType]::ParameterName, 's')
            [CompletionResult]::new('--state', '--state', [CompletionResultType]::ParameterName, 'state')
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;reject' {
            [CompletionResult]::new('--to', '--to', [CompletionResultType]::ParameterName, 'Existing destination issue')
            [CompletionResult]::new('-p', '-p', [CompletionResultType]::ParameterName, 'Project for a newly created replacement')
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'Project for a newly created replacement')
            [CompletionResult]::new('--reason', '--reason', [CompletionResultType]::ParameterName, 'Why this issue is rejected')
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;ready' {
            [CompletionResult]::new('-p', '-p', [CompletionResultType]::ParameterName, 'p')
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'project')
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'json')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;claim' {
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--force', '--force', [CompletionResultType]::ParameterName, 'Take over a claim held by another identity')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;note' {
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;append' {
            [CompletionResult]::new('--text', '--text', [CompletionResultType]::ParameterName, 'The text to append')
            [CompletionResult]::new('--file', '--file', [CompletionResultType]::ParameterName, 'Read the text from a file; `-` reads stdin')
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'vissue;claims' {
            [CompletionResult]::new('--by', '--by', [CompletionResultType]::ParameterName, 'Only claims held by this identity')
            [CompletionResult]::new('-p', '-p', [CompletionResultType]::ParameterName, 'Only claims in this project')
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'Only claims in this project')
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Machine-readable output')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;fold' {
            [CompletionResult]::new('-p', '-p', [CompletionResultType]::ParameterName, 'Project the folded issues are created in. Auto-detected from .project-ctx.toml when omitted')
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'Project the folded issues are created in. Auto-detected from .project-ctx.toml when omitted')
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;agenda' {
            [CompletionResult]::new('-d', '-d', [CompletionResultType]::ParameterName, 'Days ahead to include')
            [CompletionResult]::new('--days', '--days', [CompletionResultType]::ParameterName, 'Days ahead to include')
            [CompletionResult]::new('-p', '-p', [CompletionResultType]::ParameterName, 'p')
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'project')
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;hygiene' {
            [CompletionResult]::new('--stale-days', '--stale-days', [CompletionResultType]::ParameterName, 'Days a claim may be held before it counts as stale')
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;whoami' {
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;waiting-on' {
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;body-excerpt' {
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;search' {
            [CompletionResult]::new('-n', '-n', [CompletionResultType]::ParameterName, 'n')
            [CompletionResult]::new('--limit', '--limit', [CompletionResultType]::ParameterName, 'limit')
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;children' {
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;ancestors' {
            [CompletionResult]::new('-d', '-d', [CompletionResultType]::ParameterName, 'd')
            [CompletionResult]::new('--depth', '--depth', [CompletionResultType]::ParameterName, 'depth')
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;impact' {
            [CompletionResult]::new('-d', '-d', [CompletionResultType]::ParameterName, 'd')
            [CompletionResult]::new('--depth', '--depth', [CompletionResultType]::ParameterName, 'depth')
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;related' {
            [CompletionResult]::new('-d', '-d', [CompletionResultType]::ParameterName, 'd')
            [CompletionResult]::new('--depth', '--depth', [CompletionResultType]::ParameterName, 'depth')
            [CompletionResult]::new('-n', '-n', [CompletionResultType]::ParameterName, 'n')
            [CompletionResult]::new('--limit', '--limit', [CompletionResultType]::ParameterName, 'limit')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'text or org; org emits links to the source headings')
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;stale' {
            [CompletionResult]::new('-d', '-d', [CompletionResultType]::ParameterName, 'd')
            [CompletionResult]::new('--days', '--days', [CompletionResultType]::ParameterName, 'days')
            [CompletionResult]::new('-p', '-p', [CompletionResultType]::ParameterName, 'p')
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'project')
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;count' {
            [CompletionResult]::new('-p', '-p', [CompletionResultType]::ParameterName, 'p')
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'project')
            [CompletionResult]::new('-s', '-s', [CompletionResultType]::ParameterName, 's')
            [CompletionResult]::new('--state', '--state', [CompletionResultType]::ParameterName, 'state')
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('-r', '-r', [CompletionResultType]::ParameterName, 'Count only actionable issues')
            [CompletionResult]::new('--ready', '--ready', [CompletionResultType]::ParameterName, 'Count only actionable issues')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;export' {
            [CompletionResult]::new('-p', '-p', [CompletionResultType]::ParameterName, 'p')
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'project')
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;tree' {
            [CompletionResult]::new('-f', '-f', [CompletionResultType]::ParameterName, 'ascii or dot')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'ascii or dot')
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;cycles' {
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;graph' {
            [CompletionResult]::new('-p', '-p', [CompletionResultType]::ParameterName, 'p')
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'project')
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;refile' {
            [CompletionResult]::new('--to', '--to', [CompletionResultType]::ParameterName, 'Target project')
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;backlinks' {
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;roadmap' {
            [CompletionResult]::new('-p', '-p', [CompletionResultType]::ParameterName, 'p')
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'project')
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;check' {
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;digest' {
            [CompletionResult]::new('-p', '-p', [CompletionResultType]::ParameterName, 'Project to include; repeat for several. Omit for every project')
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'Project to include; repeat for several. Omit for every project')
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Emit a JSON object instead of text')
            [CompletionResult]::new('-q', '-q', [CompletionResultType]::ParameterName, 'Print only the combined digest')
            [CompletionResult]::new('--quiet', '--quiet', [CompletionResultType]::ParameterName, 'Print only the combined digest')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;mirror' {
            [CompletionResult]::new('-p', '-p', [CompletionResultType]::ParameterName, 'Project to include; repeat for several. Omit for every project')
            [CompletionResult]::new('--project', '--project', [CompletionResultType]::ParameterName, 'Project to include; repeat for several. Omit for every project')
            [CompletionResult]::new('-o', '-o', [CompletionResultType]::ParameterName, 'Destination file; `-` writes to standard output')
            [CompletionResult]::new('--out', '--out', [CompletionResultType]::ParameterName, 'Destination file; `-` writes to standard output')
            [CompletionResult]::new('--check', '--check', [CompletionResultType]::ParameterName, 'Compare an existing mirror''s stamp against the tracker instead of writing. Exits 0 when fresh, 1 when stale')
            [CompletionResult]::new('-f', '-f', [CompletionResultType]::ParameterName, 'org or markdown')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'org or markdown')
            [CompletionResult]::new('-s', '-s', [CompletionResultType]::ParameterName, 'Include only this state')
            [CompletionResult]::new('--state', '--state', [CompletionResultType]::ParameterName, 'Include only this state')
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;events' {
            [CompletionResult]::new('--since', '--since', [CompletionResultType]::ParameterName, 'Only events newer than this sequence')
            [CompletionResult]::new('-n', '-n', [CompletionResultType]::ParameterName, 'Maximum events returned')
            [CompletionResult]::new('--limit', '--limit', [CompletionResultType]::ParameterName, 'Maximum events returned')
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;ping' {
            [CompletionResult]::new('--detail', '--detail', [CompletionResultType]::ParameterName, 'detail')
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;wait' {
            [CompletionResult]::new('--last', '--last', [CompletionResultType]::ParameterName, 'last')
            [CompletionResult]::new('--id', '--id', [CompletionResultType]::ParameterName, 'Issue to watch when --until-terminal is set')
            [CompletionResult]::new('--poll-ms', '--poll-ms', [CompletionResultType]::ParameterName, 'poll-ms')
            [CompletionResult]::new('--timeout-ms', '--timeout-ms', [CompletionResultType]::ParameterName, 'timeout-ms')
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--until-terminal', '--until-terminal', [CompletionResultType]::ParameterName, 'Block until the issue is DONE or CANCELLED')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;gen' {
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;projects' {
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;identity' {
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;serve' {
            [CompletionResult]::new('-s', '-s', [CompletionResultType]::ParameterName, 'Control socket path. Falls back to VISSUE_CONTROL_SOCKET, then $XDG_RUNTIME_DIR/vissue/control.sock, then ~/.vissue/run/control.sock')
            [CompletionResult]::new('--socket', '--socket', [CompletionResultType]::ParameterName, 'Control socket path. Falls back to VISSUE_CONTROL_SOCKET, then $XDG_RUNTIME_DIR/vissue/control.sock, then ~/.vissue/run/control.sock')
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('-d', '-d', [CompletionResultType]::ParameterName, 'Detach after the socket accepts. The child is placed in its own process group (not a new session) and can still receive SIGHUP from the parent terminal')
            [CompletionResult]::new('--detach', '--detach', [CompletionResultType]::ParameterName, 'Detach after the socket accepts. The child is placed in its own process group (not a new session) and can still receive SIGHUP from the parent terminal')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('stop', 'stop', [CompletionResultType]::ParameterValue, 'Signal the owner (SIGTERM, then SIGKILL) and wait')
            [CompletionResult]::new('restart', 'restart', [CompletionResultType]::ParameterValue, 'Stop, then start detached')
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'Print a live/pid/socket snapshot. Exit 0 if live, 1 otherwise')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'vissue;serve;stop' {
            [CompletionResult]::new('-s', '-s', [CompletionResultType]::ParameterName, 'Control socket path. Falls back to VISSUE_CONTROL_SOCKET, then $XDG_RUNTIME_DIR/vissue/control.sock, then ~/.vissue/run/control.sock')
            [CompletionResult]::new('--socket', '--socket', [CompletionResultType]::ParameterName, 'Control socket path. Falls back to VISSUE_CONTROL_SOCKET, then $XDG_RUNTIME_DIR/vissue/control.sock, then ~/.vissue/run/control.sock')
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;serve;restart' {
            [CompletionResult]::new('-s', '-s', [CompletionResultType]::ParameterName, 'Control socket path. Falls back to VISSUE_CONTROL_SOCKET, then $XDG_RUNTIME_DIR/vissue/control.sock, then ~/.vissue/run/control.sock')
            [CompletionResult]::new('--socket', '--socket', [CompletionResultType]::ParameterName, 'Control socket path. Falls back to VISSUE_CONTROL_SOCKET, then $XDG_RUNTIME_DIR/vissue/control.sock, then ~/.vissue/run/control.sock')
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;serve;status' {
            [CompletionResult]::new('-s', '-s', [CompletionResultType]::ParameterName, 'Control socket path. Falls back to VISSUE_CONTROL_SOCKET, then $XDG_RUNTIME_DIR/vissue/control.sock, then ~/.vissue/run/control.sock')
            [CompletionResult]::new('--socket', '--socket', [CompletionResultType]::ParameterName, 'Control socket path. Falls back to VISSUE_CONTROL_SOCKET, then $XDG_RUNTIME_DIR/vissue/control.sock, then ~/.vissue/run/control.sock')
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Machine-readable object')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;serve;help' {
            [CompletionResult]::new('stop', 'stop', [CompletionResultType]::ParameterValue, 'Signal the owner (SIGTERM, then SIGKILL) and wait')
            [CompletionResult]::new('restart', 'restart', [CompletionResultType]::ParameterValue, 'Stop, then start detached')
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'Print a live/pid/socket snapshot. Exit 0 if live, 1 otherwise')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'vissue;serve;help;stop' {
            break
        }
        'vissue;serve;help;restart' {
            break
        }
        'vissue;serve;help;status' {
            break
        }
        'vissue;serve;help;help' {
            break
        }
        'vissue;tui' {
            [CompletionResult]::new('-s', '-s', [CompletionResultType]::ParameterName, 'Control socket path. Falls back to VISSUE_CONTROL_SOCKET, then $XDG_RUNTIME_DIR/vissue/control.sock, then ~/.vissue/run/control.sock')
            [CompletionResult]::new('--socket', '--socket', [CompletionResultType]::ParameterName, 'Control socket path. Falls back to VISSUE_CONTROL_SOCKET, then $XDG_RUNTIME_DIR/vissue/control.sock, then ~/.vissue/run/control.sock')
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Never attach, never spawn serve; CatalogService plus generation poll')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'vissue;hud' {
            [CompletionResult]::new('--mode', '--mode', [CompletionResultType]::ParameterName, 'ready, list (all), claims, stale, or new. Used by `--rofi`')
            [CompletionResult]::new('-s', '-s', [CompletionResultType]::ParameterName, 'Control socket path. Falls back to VISSUE_CONTROL_SOCKET, then $XDG_RUNTIME_DIR/vissue/control.sock, then ~/.vissue/run/control.sock')
            [CompletionResult]::new('--socket', '--socket', [CompletionResultType]::ParameterName, 'Control socket path. Falls back to VISSUE_CONTROL_SOCKET, then $XDG_RUNTIME_DIR/vissue/control.sock, then ~/.vissue/run/control.sock')
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--offline', '--offline', [CompletionResultType]::ParameterName, 'Never attach, never spawn serve')
            [CompletionResult]::new('--toggle', '--toggle', [CompletionResultType]::ParameterName, 'Show or hide a running board, or dismiss a live rofi picker')
            [CompletionResult]::new('--show', '--show', [CompletionResultType]::ParameterName, 'Show a running board')
            [CompletionResult]::new('--hide', '--hide', [CompletionResultType]::ParameterName, 'Hide a running board, or dismiss a live rofi picker')
            [CompletionResult]::new('--iced', '--iced', [CompletionResultType]::ParameterName, 'Use the iced board. Default when `--rofi` is absent')
            [CompletionResult]::new('--rofi', '--rofi', [CompletionResultType]::ParameterName, 'Use the rofi picker instead of the iced board')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'vissue;completions' {
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'vissue;man' {
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'vissue;keys' {
            [CompletionResult]::new('--root', '--root', [CompletionResultType]::ParameterName, 'Tracker root. Falls back to ISSUE_ROOT, VISSUE_ROOT, then the current directory')
            [CompletionResult]::new('--prefix', '--prefix', [CompletionResultType]::ParameterName, 'Directory under the root holding one subdirectory per project. Falls back to VISSUE_PREFIX, then `prefix` in vissue.toml, then `Software`')
            [CompletionResult]::new('--check', '--check', [CompletionResultType]::ParameterName, 'Load the overlay and exit 1 on conflict')
            [CompletionResult]::new('--occupancy', '--occupancy', [CompletionResultType]::ParameterName, 'Print taken chords')
            [CompletionResult]::new('--no-route', '--no-route', [CompletionResultType]::ParameterName, 'Ignore `$VISSUE_CONFIG` / `~/.config/vissue/config.toml` and keep every verb on the process default layout')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'vissue;help' {
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Create an issue. Pass the body with --body or --body-file (`-` reads stdin); omit both to leave the body empty for a later edit')
            [CompletionResult]::new('q', 'q', [CompletionResultType]::ParameterValue, 'Quick capture: create and print only the id')
            [CompletionResult]::new('list', 'list', [CompletionResultType]::ParameterValue, 'List issues, sorted by priority then state then id')
            [CompletionResult]::new('show', 'show', [CompletionResultType]::ParameterValue, 'Show one issue: metadata, then the body')
            [CompletionResult]::new('update', 'update', [CompletionResultType]::ParameterValue, 'Update state, priority, or blocker edges')
            [CompletionResult]::new('resolve', 'resolve', [CompletionResultType]::ParameterValue, 'Pick one terminal after a sibling close')
            [CompletionResult]::new('reject', 'reject', [CompletionResultType]::ParameterValue, 'Reject an issue, redirecting to an existing destination or a new replacement')
            [CompletionResult]::new('ready', 'ready', [CompletionResultType]::ParameterValue, 'Actionable issues: TODO or STARTED with no open blocker')
            [CompletionResult]::new('claim', 'claim', [CompletionResultType]::ParameterValue, 'Take an issue: move it to STARTED and stamp the claim')
            [CompletionResult]::new('note', 'note', [CompletionResultType]::ParameterValue, 'Add a dated note to the top of an issue''s logbook; state and claim untouched')
            [CompletionResult]::new('append', 'append', [CompletionResultType]::ParameterValue, 'Append a dated report to an issue''s body')
            [CompletionResult]::new('claims', 'claims', [CompletionResultType]::ParameterValue, 'Every live claim, oldest first: who holds what, and for how long')
            [CompletionResult]::new('fold', 'fold', [CompletionResultType]::ParameterValue, 'Fold an inbox org file: each unstamped `* TODO <title>` heading becomes an issue, then the heading is stamped with the id and flipped to DONE in place. Already-stamped headings are skipped')
            [CompletionResult]::new('agenda', 'agenda', [CompletionResultType]::ParameterValue, 'Dated open work: deadlines and scheduled starts inside a horizon, overdue first')
            [CompletionResult]::new('hygiene', 'hygiene', [CompletionResultType]::ParameterValue, 'Checklist for agents and CI: stalled claims plus corpus validation')
            [CompletionResult]::new('whoami', 'whoami', [CompletionResultType]::ParameterValue, 'Print the identity this tracker would record on a claim')
            [CompletionResult]::new('waiting-on', 'waiting-on', [CompletionResultType]::ParameterValue, 'Issues waiting on this one')
            [CompletionResult]::new('body-excerpt', 'body-excerpt', [CompletionResultType]::ParameterValue, 'The first lines of an issue''s file range')
            [CompletionResult]::new('search', 'search', [CompletionResultType]::ParameterValue, 'Substring search over ids, titles, properties, and bodies')
            [CompletionResult]::new('children', 'children', [CompletionResultType]::ParameterValue, 'Issues whose `:PARENT:` matches this id')
            [CompletionResult]::new('ancestors', 'ancestors', [CompletionResultType]::ParameterValue, 'Blockers transitively required by this issue')
            [CompletionResult]::new('impact', 'impact', [CompletionResultType]::ParameterValue, 'Issues transitively waiting on this issue')
            [CompletionResult]::new('related', 'related', [CompletionResultType]::ParameterValue, 'Explain bounded Org and lexical connections around an issue')
            [CompletionResult]::new('stale', 'stale', [CompletionResultType]::ParameterValue, 'Open issues whose `:CREATED:` is older than N days')
            [CompletionResult]::new('count', 'count', [CompletionResultType]::ParameterValue, 'Print only the matching issue count')
            [CompletionResult]::new('export', 'export', [CompletionResultType]::ParameterValue, 'One JSON object per issue per line')
            [CompletionResult]::new('tree', 'tree', [CompletionResultType]::ParameterValue, 'Children and blockers below an id')
            [CompletionResult]::new('cycles', 'cycles', [CompletionResultType]::ParameterValue, 'Cycles in the blocker graph')
            [CompletionResult]::new('graph', 'graph', [CompletionResultType]::ParameterValue, 'The blocker and parent graph as Graphviz DOT')
            [CompletionResult]::new('refile', 'refile', [CompletionResultType]::ParameterValue, 'Move an issue to another project''s file')
            [CompletionResult]::new('backlinks', 'backlinks', [CompletionResultType]::ParameterValue, 'Issues referring to this id')
            [CompletionResult]::new('roadmap', 'roadmap', [CompletionResultType]::ParameterValue, 'A markdown roadmap of active and closed work')
            [CompletionResult]::new('check', 'check', [CompletionResultType]::ParameterValue, 'Validate the corpus. Exits non-zero on any error')
            [CompletionResult]::new('digest', 'digest', [CompletionResultType]::ParameterValue, 'A content digest of the corpus, for telling whether a copy is current')
            [CompletionResult]::new('mirror', 'mirror', [CompletionResultType]::ParameterValue, 'Write a read-only projection of one or more projects to a file')
            [CompletionResult]::new('events', 'events', [CompletionResultType]::ParameterValue, 'Change events with a sequence above --since')
            [CompletionResult]::new('ping', 'ping', [CompletionResultType]::ParameterValue, 'Append a manual event, waking pollers without editing an issue')
            [CompletionResult]::new('wait', 'wait', [CompletionResultType]::ParameterValue, 'Block until the generation passes --last, or until an issue is terminal. Exits 2 on timeout')
            [CompletionResult]::new('gen', 'gen', [CompletionResultType]::ParameterValue, 'Print the current generation counter')
            [CompletionResult]::new('projects', 'projects', [CompletionResultType]::ParameterValue, 'List the projects found under the layout prefix')
            [CompletionResult]::new('identity', 'identity', [CompletionResultType]::ParameterValue, 'Print the resolved binary, root, and prefix')
            [CompletionResult]::new('serve', 'serve', [CompletionResultType]::ParameterValue, 'Own the per-user Unix control socket')
            [CompletionResult]::new('tui', 'tui', [CompletionResultType]::ParameterValue, 'Interactive board over ready, list, claims, agenda, and search')
            [CompletionResult]::new('hud', 'hud', [CompletionResultType]::ParameterValue, 'Task board. Default execs `vissue-hud` (Ready / Mine / Upcoming / All)')
            [CompletionResult]::new('completions', 'completions', [CompletionResultType]::ParameterValue, 'Write a shell completion script to stdout')
            [CompletionResult]::new('man', 'man', [CompletionResultType]::ParameterValue, 'Write the roff manual page to stdout')
            [CompletionResult]::new('keys', 'keys', [CompletionResultType]::ParameterValue, 'Print the HUD key catalog, or check a keys.toml overlay')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'vissue;help;create' {
            break
        }
        'vissue;help;q' {
            break
        }
        'vissue;help;list' {
            break
        }
        'vissue;help;show' {
            break
        }
        'vissue;help;update' {
            break
        }
        'vissue;help;resolve' {
            break
        }
        'vissue;help;reject' {
            break
        }
        'vissue;help;ready' {
            break
        }
        'vissue;help;claim' {
            break
        }
        'vissue;help;note' {
            break
        }
        'vissue;help;append' {
            break
        }
        'vissue;help;claims' {
            break
        }
        'vissue;help;fold' {
            break
        }
        'vissue;help;agenda' {
            break
        }
        'vissue;help;hygiene' {
            break
        }
        'vissue;help;whoami' {
            break
        }
        'vissue;help;waiting-on' {
            break
        }
        'vissue;help;body-excerpt' {
            break
        }
        'vissue;help;search' {
            break
        }
        'vissue;help;children' {
            break
        }
        'vissue;help;ancestors' {
            break
        }
        'vissue;help;impact' {
            break
        }
        'vissue;help;related' {
            break
        }
        'vissue;help;stale' {
            break
        }
        'vissue;help;count' {
            break
        }
        'vissue;help;export' {
            break
        }
        'vissue;help;tree' {
            break
        }
        'vissue;help;cycles' {
            break
        }
        'vissue;help;graph' {
            break
        }
        'vissue;help;refile' {
            break
        }
        'vissue;help;backlinks' {
            break
        }
        'vissue;help;roadmap' {
            break
        }
        'vissue;help;check' {
            break
        }
        'vissue;help;digest' {
            break
        }
        'vissue;help;mirror' {
            break
        }
        'vissue;help;events' {
            break
        }
        'vissue;help;ping' {
            break
        }
        'vissue;help;wait' {
            break
        }
        'vissue;help;gen' {
            break
        }
        'vissue;help;projects' {
            break
        }
        'vissue;help;identity' {
            break
        }
        'vissue;help;serve' {
            [CompletionResult]::new('stop', 'stop', [CompletionResultType]::ParameterValue, 'Signal the owner (SIGTERM, then SIGKILL) and wait')
            [CompletionResult]::new('restart', 'restart', [CompletionResultType]::ParameterValue, 'Stop, then start detached')
            [CompletionResult]::new('status', 'status', [CompletionResultType]::ParameterValue, 'Print a live/pid/socket snapshot. Exit 0 if live, 1 otherwise')
            break
        }
        'vissue;help;serve;stop' {
            break
        }
        'vissue;help;serve;restart' {
            break
        }
        'vissue;help;serve;status' {
            break
        }
        'vissue;help;tui' {
            break
        }
        'vissue;help;hud' {
            break
        }
        'vissue;help;completions' {
            break
        }
        'vissue;help;man' {
            break
        }
        'vissue;help;keys' {
            break
        }
        'vissue;help;help' {
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}

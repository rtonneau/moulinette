# Session Summary: Adding new list mode file conversion. Example in 'D:\OneDrive - Université de Namur\JupyterLab\PFAS/PIGE/test_mpa3kit.ipynb' using custom mpa3kit python library accessible at 'C:\DEV\python\mpa3kit'. The soft should be able to load .lst files and also make some plot of acquisition rate versus time (like in cell 11) and also violin plots like cell 13. If possible to use a similar approach as 'import ruptures as rpt' to detect changes of rate, it is a plus

**Session ID:** 2026-10-20__adding-new-list-mode-file-conversion-example-in-d-onedrive
**Created:** 2026-10-20T08:26:10.876Z
**Finished:** 2026-10-20T10:13:13.275Z
**Status:** Complete

## Grill

- Resume: [01-grill/resume.md](01-grill/resume.md)

## Plan

- Plan: [02-plan/plan.md](02-plan/plan.md)

## Tickets

- ✅ 01 lst-binary-parser — [spec](02-plan/tickets/01-lst-binary-parser.md) · [log](03-implement/01-lst-binary-parser/commit-log.md)
- ✅ 02 roi-and-time-binned-rate — [spec](02-plan/tickets/02-roi-and-time-binned-rate.md) · [log](03-implement/02-roi-and-time-binned-rate/commit-log.md)
- ✅ 03 rate-vs-time-plot-and-cli — [spec](02-plan/tickets/03-rate-vs-time-plot-and-cli.md) · [log](03-implement/03-rate-vs-time-plot-and-cli/commit-log.md)
- ✅ 04 violin-plot — [spec](02-plan/tickets/04-violin-plot.md) · [log](03-implement/04-violin-plot/commit-log.md)
- ✅ 05 gui-list-mode-panel — [spec](02-plan/tickets/05-gui-list-mode-panel.md) · [log](03-implement/05-gui-list-mode-panel/commit-log.md)

## Changelog

- **Bump:** minor
- **Fragment:** [2026-10-20__adding-new-list-mode-file-conversion-example-in-d-onedrive.md](../../changelog/2026-10-20__adding-new-list-mode-file-conversion-example-in-d-onedrive.md) (merged into the CHANGELOG at release)

## Branch & PR

- **Branch:** `feat/list-mode-analysis`
- **Base:** `main`
- **Pull request:** https://github.com/rtonneau/moulinette/pull/1

## Timeline

| When | Phase | Event | Details | Files |
|---|---|---|---|---|
| 2026-10-20 10:26 | grill | session_started |  | [01-grill/resume.md](01-grill/resume.md) |
| 2026-10-20 10:56 | plan-not-started | grill_written |  | [01-grill/resume.md](01-grill/resume.md) |
| 2026-10-20 10:56 | plan | plan_started |  | [02-plan/plan.md](02-plan/plan.md) |
| 2026-10-20 11:02 | plan | branch_created | branch: feat/list-mode-analysis, base: main |  |
| 2026-10-20 11:02 | ship | plan_written | tickets: 5 | [02-plan/plan.md](02-plan/plan.md), [02-plan/tickets/01-lst-binary-parser.md](02-plan/tickets/01-lst-binary-parser.md), [02-plan/tickets/02-roi-and-time-binned-rate.md](02-plan/tickets/02-roi-and-time-binned-rate.md), [02-plan/tickets/03-rate-vs-time-plot-and-cli.md](02-plan/tickets/03-rate-vs-time-plot-and-cli.md), [02-plan/tickets/04-violin-plot.md](02-plan/tickets/04-violin-plot.md), [02-plan/tickets/05-gui-list-mode-panel.md](02-plan/tickets/05-gui-list-mode-panel.md) |
| 2026-10-20 11:03 | ship | ticket_started | ticket: 01-lst-binary-parser | [02-plan/tickets/01-lst-binary-parser.md](02-plan/tickets/01-lst-binary-parser.md), [03-implement/01-lst-binary-parser/commit-log.md](03-implement/01-lst-binary-parser/commit-log.md) |
| 2026-10-20 11:14 | ship | ticket_done | ticket: 01-lst-binary-parser, commit: 03d11d5 | [03-implement/01-lst-binary-parser/commit-log.md](03-implement/01-lst-binary-parser/commit-log.md) |
| 2026-10-20 11:14 | ship | ticket_started | ticket: 02-roi-and-time-binned-rate | [02-plan/tickets/02-roi-and-time-binned-rate.md](02-plan/tickets/02-roi-and-time-binned-rate.md), [03-implement/02-roi-and-time-binned-rate/commit-log.md](03-implement/02-roi-and-time-binned-rate/commit-log.md) |
| 2026-10-20 11:18 | ship | ticket_done | ticket: 02-roi-and-time-binned-rate, commit: 1dc972c | [03-implement/02-roi-and-time-binned-rate/commit-log.md](03-implement/02-roi-and-time-binned-rate/commit-log.md) |
| 2026-10-20 11:18 | ship | ticket_started | ticket: 03-rate-vs-time-plot-and-cli | [02-plan/tickets/03-rate-vs-time-plot-and-cli.md](02-plan/tickets/03-rate-vs-time-plot-and-cli.md), [03-implement/03-rate-vs-time-plot-and-cli/commit-log.md](03-implement/03-rate-vs-time-plot-and-cli/commit-log.md) |
| 2026-10-20 11:40 | ship | ticket_done | ticket: 03-rate-vs-time-plot-and-cli, commit: ef6ab90 | [03-implement/03-rate-vs-time-plot-and-cli/commit-log.md](03-implement/03-rate-vs-time-plot-and-cli/commit-log.md) |
| 2026-10-20 11:40 | ship | ticket_started | ticket: 04-violin-plot | [02-plan/tickets/04-violin-plot.md](02-plan/tickets/04-violin-plot.md), [03-implement/04-violin-plot/commit-log.md](03-implement/04-violin-plot/commit-log.md) |
| 2026-10-20 11:50 | ship | ticket_done | ticket: 04-violin-plot, commit: ec2e390 | [03-implement/04-violin-plot/commit-log.md](03-implement/04-violin-plot/commit-log.md) |
| 2026-10-20 11:50 | ship | ticket_started | ticket: 05-gui-list-mode-panel | [02-plan/tickets/05-gui-list-mode-panel.md](02-plan/tickets/05-gui-list-mode-panel.md), [03-implement/05-gui-list-mode-panel/commit-log.md](03-implement/05-gui-list-mode-panel/commit-log.md) |
| 2026-10-20 12:12 | finish-pending | ticket_done | ticket: 05-gui-list-mode-panel, commit: 8e72847 | [03-implement/05-gui-list-mode-panel/commit-log.md](03-implement/05-gui-list-mode-panel/commit-log.md) |
| 2026-10-20 12:13 | finish-pending | changelog_written | bump: minor |  |
| 2026-10-20 12:13 | finish-pending | pr_opened | url: https://github.com/rtonneau/moulinette/pull/1 |  |
| 2026-10-20 12:13 | finished | session_finished |  | [INDEX.md](INDEX.md) |

## Next

Start a new feature with /gps start <next-feature>

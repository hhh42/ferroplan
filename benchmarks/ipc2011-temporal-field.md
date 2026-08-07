# IPC temporal-track field results (2008 / 2011 / 2014), vendored 2026-08-07

Every field number below was earned at the competitions' **30-MINUTE
per-instance budget** (IPC-2008: 1800 s / 2 GB; IPC-2011: 1800 s / 6 GB;
IPC-2014: 1800 s / 4 GB) — a **60× larger budget** than ferroplan's 30 s
tier, on decade-older hardware. This file prices the 110-instance temporal
zero block (storage-t 2×20, temporal-machine-shop 2×20, model-train 30)
against that field. Provenance is mixed and labeled per row, because the
official per-instance archives for 2008 and 2011 are lost: the IPC-2011
site (plg.inf.uc3m.es) is DNS-dead, its svn server (pleiades) is gone, and
the Wayback Machine captured the results *pages* but never the results
*tarballs* (`ipc2011-results.tar.bz2`, `ipc2011-snapshots.tar.bz2`,
`ipc2008-results.tar.bz2` — CDX-verified absent, 2026-08-07). What survives:
the organizers' AIJ-2015 report [1], the official results talk [2] (vendored
under `ipc-2011t-results/`), the IPC-2014 organizers' KER report [4] and
results talk [5], and two peer-reviewed 30-minute re-runs on the same
instance sets [6][7]. Where only an IPC quality score is published, coverage
is reported as `≥ceil(score)` (each solved instance contributes ≤1 to score).

## The zero-block domains

| domain | set | best-of-field coverage @30 min | field notes | source |
|---|---|---|---|---|
| storage-t | ipc-2011 (20) | counts lost; top score band | official talk heat-tables put DAEYAHSP and YAHSP2-MT in the top score band on STORAGE, YAHSP2 mid, POPF2/LMTD/CPT4 pale-to-zero; exact per-domain counts were only in the lost tarballs | [1] §4.5, [2] pp. 88–98 |
| storage-t | ipc-2014 (20) | **20/20** (ITSAT, SCP2; LPG-td 18/20) | official field-average ≈18% solved (Fig. 4); OPTIC 0/20 and TFD 0/20 — half the field's planner styles also score 0 here | [4] Fig. 4, [6] Table 6 |
| temporal-machine-shop (tms) | ipc-2011 (20) | **5/20** (POPF2) | required concurrency; 6 of 8 entrants at 0 valid: LMTD 0, epoch-based trio (DAEYAHSP/YAHSP2/YAHSP2-MT) produced only VAL-invalid plans here, SHARAABI all-invalid, TLP-GP parser-dead | [1] §4.5.1 |
| temporal-machine-shop (tms) | ipc-2014 (20) | **18/20** (ITSAT, official domain score 18.0) | all five other entrants 0 valid (yahsp-family plans VAL-invalid on the concurrency trio; TFD/tBURTON 0); JAIR re-run confirms ITSAT 18/20, all others 0 | [4] §4.5.2, [6] Table 6 |
| model-train | ipc-2008 (30) | **≥12/30** (BasePlanner, quality score 11.92) | SGPlan6 (official winner) score 11.11 → ≥12/30; TFD 0.96 → ≈1; Crikey3, LPG-td, Sapa errored/no valid data. BasePlanner = organizers' baseline: sequential Metric-FF + rescheduling — model-train needs no concurrency, it falls to plain reachability + size | [7] Table 1 |

## The sitting's other families

| domain | set | best-of-field coverage @30 min | field notes | source |
|---|---|---|---|---|
| floor-tile-t | ipc-2011 (20) | counts lost | POPF2 solved 0 here (parser bug, fixed post-competition); yahsp family scored | [1] §4.5.3 fn.14, [2] |
| floor-tile-t | ipc-2014 (20) | 20/20 (ITSAT, LPG-td) | official: ITSAT domain score 18.8; field-average ≈30% | [4] §4.5.2, [6] Table 6 |
| sokoban-t | ipc-2011 (20) | 8/20 (ITSAT re-run; official counts lost) | re-run: OPTIC 2, TFD 3, LPG-td 5, SCP2 1 | [6] Table 6 |
| turn-and-open | ipc-2011 (20) | **13/20** (LMTD; POPF2 9/20) | required concurrency; epoch-based trio 0 valid | [1] §4.5.1 |
| turn-and-open | ipc-2014 (20) | 18/20 (TFD re-run) | official field-average ≈22%; re-run: ITSAT 9, OPTIC 9, LPG-td 0, SCP2 1 | [4] Fig. 4, [6] Table 6 |
| parc-printer-t | ipc-2011 (20) | 20/20 (ITSAT re-run; official counts lost) | re-run: OPTIC 0, TFD 0, LPG-td 7, SCP2 0 | [6] Table 6 |
| driver-log-t | ipc-2014 (20) | 14/20 (LPG-td re-run) | official field-average ≈12% (hard by SIZE, not concurrency, per organizers); re-run: ITSAT 4, OPTIC 0, TFD 0 | [4] §3.3, [6] Table 6 |
| satellite-t | ipc-2014 (20) | 20/20 (LPG-td re-run) | official field-average ≈64%; re-run: ITSAT 16, TFD 13, OPTIC 4 | [4] Fig. 4, [6] Table 6 |
| map-analyzer-t | ipc-2014 (20) | 20/20 (ITSAT, LPG-td re-run) | official field-average ≈63%; re-run: TFD 19, OPTIC 0 | [4] Fig. 4, [6] Table 6 |
| road-traffic-accident-mgmt (rtam) | ipc-2014 (20) | 20/20 (LPG-td re-run) | official field-average ≈50%; Temporal-FD officially 0 here (PDDL issue, [4] §4.5.3); re-run: ITSAT 0, OPTIC 0, TFD 0 | [4] Fig. 4, [6] Table 6 |
| match-cellar | ipc-2011 (20) | **20/20** (POPF2) | LMTD 15/20; epoch-based trio 0 valid | [1] §4.5.1 |
| match-cellar | ipc-2014 (20) | 20/20 (ITSAT, OPTIC, TFD re-run) | official: ITSAT domain score 19.0; field-average ≈33% | [4] §4.5.2, [6] Table 6 |
| peg-solitaire-t | ipc-2011 (20) | 20/20 (ITSAT, LPG-td, SCP2 re-run; official counts lost) | | [6] Table 6 |
| crew-planning-t | ipc-2011 (20) | 20/20 (ITSAT, OPTIC re-run; official counts lost) | | [6] Table 6 |
| openstacks-t | ipc-2011 (20) | 20/20 (OPTIC, TFD, LPG-td re-run) | | [6] Table 6 |
| elevators-t | ipc-2011 (20) | 9/20 (LPG-td re-run; official counts lost — official yahsp-family shading is top-band, likely higher) | re-run: ITSAT 0, OPTIC 1, TFD 3 | [2], [6] Table 6 |
| parking-t | ipc-2011 (20) | counts lost | | [2] |
| parking-t | ipc-2014 (20) | 20/20 (TFD, LPG-td re-run) | official field-average ≈59% | [4] Fig. 4, [6] Table 6 |
| crew-planning / elevators / openstacks / parc-printer / peg-solitaire / sokoban / transport / woodworking | ipc-2008 (30 each) | ≥29 / ≥23 / ≥27 / ≥19 / ≥28 / ≥16 / ≥12 / ≥27 (of 30) | lower bounds via best per-domain IPC quality score in the TFD re-run at the competition budget (crewpl TFD 28.72, elev LPG-td 22.75, openst TFD 26.66, parcpr LPG-td 18.20, pegsol TFD 27.57, sokoban BasePlanner 15.52, transport LPG-td 11.57, woodw LPG-td 26.37) | [7] Table 1 |

## Official track totals (for scale)

IPC-2011 temporal satisficing, 12 domains × 20 = 240 [1, Table 7]:

| planner | IPC score | solved/240 |
|---|---|---|
| DAEYAHSP (winner) | 126.16 | 136 |
| YAHSP2-MT (runner-up ex aequo) | 111.14 | 145 |
| POPF2 (runner-up ex aequo) | 110.60 | 119 |
| YAHSP2 | 98.97 | 137 |
| LMTD | 57.75 | 62 |
| CPT4 | 44.41 | 46 |
| SHARAABI | 0 | 0 (63 claimed, all VAL-invalid) |
| TLP-GP | 0 | 0 (parser bug) |

Up to 40.1% of all plans submitted in this track were VAL-invalid, mostly
the yahsp family on the three required-concurrency domains [1, §4.5.1].
Budget-relevant: YAHSP2-MT had 144 of its 145 solves by t = 196 s; the
remaining 1604 s bought exactly one more instance [1, §4.5.2].

IPC-2014 temporal satisficing, 10 domains × 20 = 200 [4, Table 7]:

| planner | IPC score | solved/200 |
|---|---|---|
| YAHSP3-MT (winner) | 86.5 | 97 |
| Temporal-FD (runner-up) | 79.2 | 94 |
| YAHSP3 | 66.6 | 103 |
| ITSAT | 65.6 | 71 |
| DAE-YAHSP | 55.0 | 75 |
| tBURTON | 0.0 | 0 |

## Sources

1. C. Linares López, S. Jiménez Celorrio, Á. García Olaya, "The
   deterministic part of the seventh International Planning Competition",
   Artificial Intelligence 223 (2015) 82–119. Author copy:
   https://plg.uc3m.es/papers/linares-et-al-aij2015.pdf (retrieved
   2026-08-07). DOI: 10.1016/j.artint.2015.01.004.
2. IPC-2011 official results talk (ICAPS-11), organizers' slides; original
   host dead, Wayback copy retrieved 2026-08-07:
   https://web.archive.org/web/20220221174358/http://www.plg.inf.uc3m.es/ipc2011-deterministic/attachments/Results/ipc2011-talk.pdf
   — vendored at `benchmarks/ipc-2011t-results/ipc2011-talk.pdf`.
3. IPC-2011 Results page (documents the lost tarballs and the color-table
   conventions), Wayback capture 2012-06-16, retrieved 2026-08-07:
   https://web.archive.org/web/20120616033607/http://www.plg.inf.uc3m.es/ipc2011-deterministic/Results
4. M. Vallati, L. Chrpa, T.L. McCluskey, "What you always wanted to know
   about the deterministic part of the International Planning Competition
   (IPC) 2014 (but were too afraid to ask)", Knowledge Engineering Review
   33 (2018) e3. Accepted manuscript (open access), retrieved 2026-08-07:
   https://eprints.hud.ac.uk/id/eprint/33269/1/ipc-ker%281%29.pdf
   DOI: 10.1017/S0269888918000012.
5. IPC-2014 official results talk (organizers' slides), Wayback capture
   2024-03-13, retrieved 2026-08-07:
   https://web.archive.org/web/20240313144604/https://helios.hud.ac.uk/scommv/IPC-14/repository/slides.pdf
6. M.F. Rankooh, G. Ghassem-Sani, "ITSAT: An Efficient SAT-Based Temporal
   Planner", JAIR 53 (2015) 541–632, Table 6 — 30-minute re-run of ITSAT,
   OPTIC, TFD, LPG-td, SCP2 on the IPC-2011/2014 temporal sets (3.1 GHz
   i5, 4 GB). Retrieved 2026-08-07:
   https://www.jair.org/index.php/jair/article/view/10950
   DOI: 10.1613/jair.4697.
7. P. Eyerich, R. Mattmüller, G. Röger, "Using the Context-enhanced
   Additive Heuristic for Temporal and Numeric Planning", ICAPS 2009 —
   IPC-2008 temporal domains at the competition budget (30 min / 2 GB,
   2.66 GHz Xeon), Tables 1–2. Retrieved 2026-08-07:
   https://gki.informatik.uni-freiburg.de/papers/eyerich-etal-icaps09.pdf
8. IPC-2008 Results page (documents the official tarballs, unarchived),
   Wayback capture 2011-01-17, retrieved 2026-08-07:
   https://web.archive.org/web/20110117175349/http://ipc.informatik.uni-freiburg.de/Results

Searched and came up empty (2026-08-07): live plg.inf.uc3m.es and
ipc.informatik.uni-freiburg.de (DNS dead / unreachable); Wayback CDX for
all three results tarballs and for MoinMoin `do=get` attachment URLs on
both hosts; GitHub repository and code search for tarball-name mirrors
(only Asai's domain-only mirrors exist). Model-train ran in IPC-2008 only —
IPC-2011 dropped it because it requires `:numeric-fluents`, unsupported by
five of the eight 2011 temporal entrants [1, §3.1]; TMS and the temporal
storage/floortile/parking sets ran 2011 and were re-instanced for 2014.

# Windows dependencies

padwight on Windows relies on drivers that Windows doesn't ship. This page records each one: what it does, its license, and whether it's maintained. Review it on the schedule below, not only when something breaks.

## Dependencies

| Dependency | Used for | License | Status (checked 2026-10-12) |
|---|---|---|---|
| [ViGEmBus](https://github.com/nefarius/ViGEmBus) | Virtual Xbox 360 pad (`vigem-client`) | BSD-3-Clause | **Archived, unmaintained.** Last push 2023-11-02. No maintained replacement found. |
| [HidHide](https://github.com/nefarius/HidHide) | Hides the physical controller from games | MIT | Active. Last push 2026-07-05. |

Neither driver is needed to build padwight. Without ViGEmBus, keyboard and mouse mappings still work, and the pad output is dropped with a log entry. Without HidHide, games see both the physical controller and padwight's virtual one.

## Known risks

- **ViGEmBus may stop working on new Windows releases.** It's a kernel driver, and nobody is updating it. Test each new Windows 11 feature update before recommending it.
- **Licenses.** ViGEmBus is BSD-3-Clause and HidHide is MIT. Any installer that redistributes them must include their copyright notices and license texts. HidHide's copyright lines differ between `LICENSE` (2020 and 2021–2024 holders) and `LICENSE.rtf` (2020 only). Use the `LICENSE` text, which has both.

## Review schedule

Check every three months, and before each release:

- [ ] Is ViGEmBus still archived, or has it been revived?
- [ ] Is there a maintained alternative that offers an Xbox 360 virtual pad (or an Xbox One pad or DualShock 4 pad) on current Windows 11? Search GitHub for "virtual gamepad driver" and "virtual controller driver", and check whether any candidate is signed for current Windows.
- [ ] Does padwight still build and create a virtual pad on the latest Windows 11 feature update?
- [ ] Has HidHide released a new version, and does its license still match the table?

Record the result of each check in the table's Status column and the date.

## If a replacement appears

Swap the output in `src/output/windows.rs` and update `Cargo.toml`, then update this page and the README's Windows section. The README says a virtual controller needs ViGEmBus, so change that sentence as well.

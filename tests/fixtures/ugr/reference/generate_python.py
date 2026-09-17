"""Generate Python reference values for the eulumdat-core UGR parity tests.

Requires eulumdat-ugr 1.0.2 and eulumdat-luminance 1.3.1 (plus their
dependencies eulumdat-py, numpy, scipy):

    python3 -m venv /tmp/ugr-venv
    /tmp/ugr-venv/bin/pip install eulumdat-ugr==1.0.2 eulumdat-luminance==1.3.1
    /tmp/ugr-venv/bin/python tests/fixtures/ugr/reference/generate_python.py

For every tests/fixtures/ugr/sample_NN.ldt the script writes:

- ugr_table_NN_python.csv: the unrounded 19x10 UgrResult.values table,
  4 decimals, same layout as the Relux/DIALux reference tables.
- luminance_NN_python.csv: LuminanceResult.at() and projected_area() samples
  from LuminanceCalculator.compute(ldt, full=True), the result that
  eulumdat-ugr uses internally.
"""

from pathlib import Path

from eulumdat_luminance import LuminanceCalculator
from eulumdat_ugr import UgrCalculator
from pyldt import LdtReader

REFERENCE_DIR = Path(__file__).resolve().parent
FIXTURE_DIR = REFERENCE_DIR.parent

# Grid angles plus off-grid angles that exercise the bilinear interpolation
# and the C wrap-around between 345 and 360 degrees.
C_ANGLES = [0.0, 12.0, 30.0, 90.0, 200.0, 352.5]
GAMMA_ANGLES = [0.0, 45.0, 65.0, 67.0, 85.0]


def main() -> None:
    samples = sorted(FIXTURE_DIR.glob("sample_*.ldt"))
    if not samples:
        raise SystemExit(f"no sample_*.ldt files found in {FIXTURE_DIR}")

    for path in samples:
        number = path.stem.removeprefix("sample_")
        ldt = LdtReader.read(path)

        ugr = UgrCalculator.compute(ldt)
        ugr_csv = REFERENCE_DIR / f"ugr_table_{number}_python.csv"
        ugr_csv.write_text(ugr.to_csv("{:.4f}") + "\n", encoding="utf-8")

        luminance = LuminanceCalculator.compute(ldt, full=True)
        lines = ["c_deg,gamma_deg,luminance_cd_m2,projected_area_m2"]
        for c in C_ANGLES:
            for gamma in GAMMA_ANGLES:
                value = luminance.at(c, gamma)
                area = luminance.projected_area(c, gamma)
                lines.append(f"{c},{gamma},{value:.10g},{area:.10g}")
        luminance_csv = REFERENCE_DIR / f"luminance_{number}_python.csv"
        luminance_csv.write_text("\n".join(lines) + "\n", encoding="utf-8")

        print(f"{path.name}: wrote {ugr_csv.name}, {luminance_csv.name}")


if __name__ == "__main__":
    main()

import hashlib
import tempfile
import unittest
from pathlib import Path
import generate_logical_keys

FIXTURE = Path(__file__).parent / "fixtures/input-event-codes.fixture.h"

class GeneratorTests(unittest.TestCase):
    def test_provenance_and_exclusions_are_deterministic(self):
        text, rows = generate_logical_keys.parse(FIXTURE)
        self.assertEqual(hashlib.sha256(text.encode()).hexdigest(), hashlib.sha256(FIXTURE.read_bytes()).hexdigest())
        self.assertEqual(rows, [("KEY_ESC", 1, "Esc"), ("KEY_POWER", 116, "Power")])
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "generated.rs"
            generate_logical_keys.main = generate_logical_keys.main
            import subprocess, sys
            subprocess.run([sys.executable, str(Path(__file__).parent / "generate_logical_keys.py"), str(FIXTURE), str(output), "--linux-tag", "v6.16", "--source-url", "https://example.invalid/pinned.h", "--catalog-version", "1"], check=True)
            generated = output.read_text()
            self.assertIn('REGISTRY_LINUX_TAG: &str = "v6.16"', generated)
            self.assertIn('REGISTRY_SOURCE_SHA256', generated)
            self.assertIn('REGISTRY_SOURCE_URL: &str = "https://example.invalid/pinned.h"', generated)
            self.assertIn('REGISTRY_LICENSE', generated)
            self.assertNotIn('KEY_RESERVED', generated)
            self.assertNotIn('KEY_UNKNOWN', generated)
            self.assertNotIn('KEY_MAX', generated)

if __name__ == "__main__": unittest.main()

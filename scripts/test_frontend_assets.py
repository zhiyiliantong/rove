import tempfile
import unittest
from pathlib import Path

from frontend_assets import validate_frontend_assets


class FrontendAssetsTests(unittest.TestCase):
    def test_complete_and_incomplete_packages(self):
        with tempfile.TemporaryDirectory() as directory:
            index = Path(directory) / "index.html"
            binary = Path(directory) / "gui.exe"
            with self.assertRaisesRegex(ValueError, "missing frontend"):
                validate_frontend_assets(index, binary)
            index.write_text('<script src="/assets/index-new.js"></script><link href="/assets/style.css">')
            with self.assertRaisesRegex(ValueError, "missing GUI"):
                validate_frontend_assets(index, binary)
            for content in (b"MZ asset not found: index.html", b"MZ assets/index-old.js", b"MZ assets/index-new.js"):
                binary.write_bytes(content)
                with self.assertRaisesRegex(ValueError, "does not embed"):
                    validate_frontend_assets(index, binary)
            binary.write_bytes(b"MZ assets/index-new.js\x00assets/style.css")
            validate_frontend_assets(index, binary)
            index.write_text("<html></html>")
            with self.assertRaisesRegex(ValueError, "frontend entry"):
                validate_frontend_assets(index, binary)


if __name__ == "__main__":
    unittest.main()

"""Build a self-contained HTML inspection snapshot from the React offline bundle.
Presentation/testing utility only. Source of truth stays in src/ and Vite.
"""
from pathlib import Path
import re
ROOT = Path(__file__).resolve().parents[1]
def css_content(path: Path) -> str:
    source = path.read_text()
    def expand(m: re.Match) -> str:
        relative = m.group(1)
        if relative.startswith(('http:', 'https:')):
            return ''
        return css_content((path.parent/relative).resolve())
    return re.sub(r'@import\s+(?:url\()?\s*[\'"]([^\'"]+)[\'"]\s*\)?\s*;', expand, source)
styles = '\n'.join(css_content(ROOT/f'src/styles/{n}.css') for n in ['tokens','base','app','diagrams','gallery'])
scripts = '\n'.join('<script>'+ (ROOT/'preview'/n).read_text().replace('</script>','<\\/script>') + '</script>'
    for n in ['runtime/react.js','runtime/react-dom.js','bundle.js'])
html = ('<!doctype html><html lang="en"><head><meta charset="utf-8"/>'
        '<meta name="viewport" content="width=device-width, initial-scale=1"/>'
        '<title>StarUML · UI Study v3</title><style>'+styles+'</style></head>'
        '<body><div id="root"></div>'+scripts+'</body></html>')
(ROOT/'preview/standalone.html').write_text(html)
print('Standalone preview:',len(html),'bytes')

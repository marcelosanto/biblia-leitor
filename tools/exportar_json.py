"""Exporta assets/biblia.db -> assets/biblia.json (usado só pelo build web).
Uso(na raiz do projeto):  python3 tools/exportar_json.py
"""
import json
import sqlite3

c = sqlite3.connect("assets/biblia.db")
livros = c.execute("SELECT id, name, abbrev FROM books ORDER BY id").fetchall()
versos = c.execute(
    "SELECT book, chapter, verse, text FROM verses ORDER BY book, chapter, verse"
).fetchall()

with open("assets/biblia.json", "w", encoding="utf-8") as f:
    json.dump({"livros": livros, "versos": versos}, f, ensure_ascii=False, separators=(",", ":"))

print(f"{len(livros)} livros, {len(versos)} versículos -> assets/biblia.json")

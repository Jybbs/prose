"""
A list of CJK strings whose inline form has fewer code points than the
line-length budget but a wider visual width that exceeds it. Column
math runs through `unicode_width`, so the visual width triggers
expansion even though the code-point count would fit.
"""

cjk_cities = ["東京", "京都", "大阪", "札幌", "横浜", "神戸", "福岡", "広島", "宮島", "金沢", "仙台", "千葉"]

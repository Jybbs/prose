import os
# Intrapackage imports
from future.backports.email.encoders import bencode, qencode
from future.backports.urllib.parse import quote

joined = os.sep + quote

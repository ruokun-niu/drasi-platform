# generated by datamodel-codegen:
#   filename:  Header.yaml
#   timestamp: 2024-11-22T20:54:01+00:00

from __future__ import annotations

from pydantic import BaseModel

from .HeaderItem import HeaderItem


class Header(BaseModel):
    header: HeaderItem
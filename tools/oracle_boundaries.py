#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Precisely identified undefined paths in the pinned, unmodified C oracle."""

# file 5.48, commit 41b1123083ab0bbdfe32f6d4ec14f6aec9a91e59:
# src/cdf_time.c:cdf_timestamp_to_timespec rejects approximate years > 9999
# before assigning ts.tv_sec. src/print.c:file_fmtdatetime ignores its return
# value and reads that uninitialized member. Its output is not a date oracle.
FILETIME_UNINITIALIZED_MIN = (10000 - 1601) * 365 * 86400 * 10_000_000


def filetime_has_uninitialized_seconds(value):
    """Accept an unsigned 64-bit rule value; C interprets it as signed ticks.

    This identifies only the early year-limit return, not arbitrary mismatches
    or every possible native conversion failure. Negative signed ticks do not
    enter that branch.
    """
    if not 0 <= value < 1 << 64:
        raise ValueError('FILETIME value must fit u64')
    return FILETIME_UNINITIALIZED_MIN <= value < 1 << 63

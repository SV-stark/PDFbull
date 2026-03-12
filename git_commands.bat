@echo off
cd /d E:\PDFbull
git add -A
git status > git_status_output.txt
git diff --cached --stat >> git_status_output.txt
type git_status_output.txt

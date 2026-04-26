#!/bin/bash

APP_NAME="conduit"

echo "正在卸载 Conduit..."

rm -f ~/.local/bin/$APP_NAME
rm -f ~/.local/share/icons/conduit-icon.png
rm -f ~/.local/share/applications/$APP_NAME.desktop

echo "卸载完成。"

#!/bin/bash

# 设置名称
APP_NAME="conduit"
DISPLAY_NAME="Conduit"
ICON_NAME="conduit-icon.png"

echo "开始安装 $DISPLAY_NAME..."

# 1. 编译项目
echo "正在编译发布版本..."
cargo build --release
if [ $? -ne 0 ]; then
    echo "错误: 编译失败，请检查 Rust 环境。"
    exit 1
fi

# 2. 创建必要的目录
mkdir -p ~/.local/bin
mkdir -p ~/.local/share/icons
mkdir -p ~/.local/share/applications

# 3. 拷贝二进制文件
echo "正在安装二进制文件..."
cp target/release/$APP_NAME ~/.local/bin/

# 4. 拷贝图标
echo "正在安装图标..."
cp assets/images/Conduit-logoonly.png ~/.local/share/icons/$ICON_NAME

# 5. 创建 .desktop 文件
echo "正在生成桌面快捷方式..."
CAT_PATH=$(which cat)
$CAT_PATH <<EOF > ~/.local/share/applications/$APP_NAME.desktop
[Desktop Entry]
Name=$DISPLAY_NAME
Comment=简单易用的网络共享与端口转发工具
Exec=$HOME/.local/bin/$APP_NAME
Icon=$HOME/.local/share/icons/$ICON_NAME
Terminal=false
Type=Application
Categories=Network;Utility;
Keywords=Network;Forward;Share;
EOF

# 6. 设置权限
chmod +x ~/.local/share/applications/$APP_NAME.desktop

echo "------------------------------------------------"
echo "安装完成！"
echo "现在你可以在应用菜单中找到 '$DISPLAY_NAME' 了。"
echo "注意：如果菜单中没出现，可能需要注销并重新登录，或确保 ~/.local/bin 在你的 PATH 中。"

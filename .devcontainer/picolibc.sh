#!/usr/bin/bash
# === Configuration ===
TOOLCHAIN_PATH="/arm/15.2.1"
MULTILIB_DIR="$TOOLCHAIN_PATH/lib/gcc/arm-none-eabi/15.2.1" # Adjust version if needed
INSTALL_PREFIX="$TOOLCHAIN_PATH/arm-none-eabi/picolibc"
BUILD_DIR="/tmp/picolibc-build"

export PATH=$PATH:$TOOLCHAIN_PATH/bin
which arm-none-eabi-gcc
echo ">>> Cloning Picolibc..."
rm -rf "$BUILD_DIR"
git clone https://github.com/picolibc/picolibc.git "$BUILD_DIR"
cd "$BUILD_DIR"

# Find the default multilib directory to use as a template
MULTILIB_TEMPLATE=$(ls -d "$MULTILIB_DIR"/thumb/* 2>/dev/null | head -n 1)
if [ -z "$MULTILIB_TEMPLATE" ]; then
  echo "Could not determine multilib directory, using defaults."
  MULTILIB_TEMPLATE="/usr/lib/arm-none-eabi"
fi

echo ">>> Configuring build with Meson..."
#-Dnewlib-retargetable-locking=false \
#-Dtinystdio=true \
meson setup build \
  --cross-file scripts/cross-arm-none-eabi.txt \
  --prefix="$INSTALL_PREFIX" \
  -Dmultilib=true \
  -Dc_args='-nostdlib' \
  -Dthread-local-storage=false \
  -Dtests=false

echo ">>> Building Picolibc..."
ninja -C build

echo ">>> Installing Picolibc to $INSTALL_PREFIX..."
mkdir -p ${TOOLCHAIN_PATH}/arm-none-eabi/picolibc/lib/
ninja -C build install

echo ">>> Setting up specs file for easy access..."
SPECS_SRC="$INSTALL_PREFIX/lib/gcc/arm-none-eabi/15.2.1/picolibc.specs"
GCC_SPECS_DIR="$TOOLCHAIN_PATH/lib/gcc/arm-none-eabi/15.2.1"

# Copy the specs file where GCC can find it by short name
cp "$SPECS_SRC" "$GCC_SPECS_DIR/picolibc.specs"

# The installed specs file likely has hardcoded paths pointing to INSTALL_PREFIX.
# We need to ensure those paths are correct for your setup.
# Usually, 'sed' is used to replace build-time paths with the final paths.
sed -i "s|$INSTALL_PREFIX|$INSTALL_PREFIX|g" "$GCC_SPECS_DIR/picolibc.specs"

echo ">>> Done!"
echo "Picolibc has been installed and integrated."
echo "You can now use it with: arm-none-eabi-gcc --specs=picolibc.specs ..."

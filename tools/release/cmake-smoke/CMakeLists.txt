# Smoke test of the CMake package of the libproj archive (tools/release-smoke.sh and .ps1): a project
# finds PROJ as it would find PROJ 9.9.0, links PROJ::proj, and builds tools/release/smoke.c.
cmake_minimum_required(VERSION 3.16)
project(proj_rs_cmake_smoke C)

find_package(PROJ 9.9 CONFIG REQUIRED)
message(STATUS "PROJ_VERSION=${PROJ_VERSION}")
message(STATUS "PROJ_LIBRARIES=${PROJ_LIBRARIES}")

add_executable(smoke "${CMAKE_CURRENT_LIST_DIR}/../smoke.c")
target_link_libraries(smoke PRIVATE PROJ::proj)

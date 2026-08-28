cmake_minimum_required(VERSION 3.21)
project(souvenir VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

add_library(souvenir src/engine.cpp src/api.cpp)
add_library(souvenir::souvenir ALIAS souvenir)
# static lib links into the Python shared module — must be PIC on Linux/GCC
set_target_properties(souvenir PROPERTIES POSITION_INDEPENDENT_CODE ON)
target_compile_features(souvenir PUBLIC cxx_std_20)
target_include_directories(souvenir PUBLIC
  $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
  $<INSTALL_INTERFACE:include>)
# vendored nlohmann/json — used in the interface only via to_json/from_json strings
target_include_directories(souvenir SYSTEM PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/third_party)

if(MSVC)
  target_compile_options(souvenir PRIVATE /W4)
else()
  target_compile_options(souvenir PRIVATE -Wall -Wextra -Wpedantic)
endif()

include(FetchContent)

option(SOUVENIR_BUILD_PYTHON "Build the Python bindings (module copied to repo root)" OFF)

if(SOUVENIR_BUILD_PYTHON)
  FetchContent_Declare(pybind11
    GIT_REPOSITORY https://github.com/pybind/pybind11.git
    GIT_TAG v3.0.1 GIT_SHALLOW TRUE)
  FetchContent_MakeAvailable(pybind11)

  pybind11_add_module(souvenir_py bindings/souvenir_py.cpp)
  target_link_libraries(souvenir_py PRIVATE souvenir)
  set_target_properties(souvenir_py PROPERTIES OUTPUT_NAME souvenir)
  # local-build distribution: drop the module next to cli.py/tui.py
  add_custom_command(TARGET souvenir_py POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy $<TARGET_FILE:souvenir_py> ${CMAKE_CURRENT_SOURCE_DIR}/..)
endif()

option(SOUVENIR_BUILD_TESTS "" ${PROJECT_IS_TOP_LEVEL})

if(SOUVENIR_BUILD_TESTS)
  enable_testing()
  FetchContent_Declare(googletest
    GIT_REPOSITORY https://github.com/google/googletest.git
    GIT_TAG v1.15.2 GIT_SHALLOW TRUE)
  set(gtest_force_shared_crt ON CACHE BOOL "" FORCE)
  FetchContent_MakeAvailable(googletest)

  add_executable(souvenir_tests tests/engine_tests.cpp)
  target_link_libraries(souvenir_tests PRIVATE souvenir GTest::gtest_main)
  target_include_directories(souvenir_tests SYSTEM PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/third_party)
  include(GoogleTest)
  gtest_discover_tests(souvenir_tests)
endif()

cmake_minimum_required(VERSION 3.16)
project(loadpath LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

add_library(loadpath src/cpm.cpp)
target_include_directories(loadpath PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)

enable_testing()
add_executable(test_cpm tests/test_cpm.cpp)
target_link_libraries(test_cpm PRIVATE loadpath)
add_test(NAME cpm COMMAND test_cpm)
set_tests_properties(cpm PROPERTIES WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}")

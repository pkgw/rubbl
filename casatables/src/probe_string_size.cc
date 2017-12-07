// Copyright 2017 Peter Williams <peter@newton.cx> and collaborators
// Licensed under the MIT License.
//
// Sigh. See build.rs for an explanation of what we're doing here.

#include <iostream>
#include <casacore/casa/BasicSL.h>

int
main(int argc, char **argv)
{
    std::cout << sizeof(casacore::String) << std::endl;
    return 0;
}

// Minimal stub implementations for casacore TableUtil functions
// to satisfy linking against the subset of casacore we build in
// rubbl_casatables_impl. These provide basic functionality used
// by our example and can be replaced with full implementations
// if TableUtil.cc is added to the vendor set.

#include <casacore/tables/Tables/Table.h>
#include <casacore/tables/Tables/TableLock.h>
#include <casacore/tables/DataMan/TSMOption.h>
#include <casacore/tables/Tables/TableDesc.h>
#include <casacore/tables/Tables/TableInfo.h>
#include <casacore/casa/Exceptions/Error.h>

using namespace casacore;

namespace casacore {
namespace TableUtil {

Table openTable(const String& tableName,
                Table::TableOption option,
                const TSMOption& tsmOption) {
    // Basic open; ignores tsmOption in this stub
    return Table(tableName, option, tsmOption);
}

Table openTable(const String& tableName,
                const TableLock& lockOptions,
                Table::TableOption option,
                const TSMOption& tsmOption) {
    // Basic open with lock options
    return Table(tableName, lockOptions, option, tsmOption);
}

Bool canDeleteTable(const String& /*tableName*/, Bool /*checkSubTables*/) {
    // Conservative default: allow deletion
    return True;
}

Bool canDeleteTable(String& message, const String& /*tableName*/,
                    Bool /*checkSubTables*/, Bool /*splitColons*/) {
    message = String();
    return True;
}

void deleteTable(const String& tableName, Bool /*checkSubTables*/) {
    // Use Table::markForDelete via opening the table and marking
    Table t(tableName, Table::Update);
    t.markForDelete();
}

rownr_t getLayout(TableDesc& /*desc*/, const String& tableName) {
    Table t(tableName, Table::Old);
    return t.nrow();
}

TableInfo tableInfo(const String& tableName) {
    Table t(tableName, Table::Old);
    return t.tableInfo();
}

} // namespace TableUtil
} // namespace casacore



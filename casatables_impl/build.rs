// Copyright 2017-2021 Peter Williams <peter@newton.cx> and collaborators
// Licensed under the MIT License.

use std::{env, fs, path::PathBuf};

fn main() {
    cc::Build::new()
        .cpp(true)
        .warnings(true)
        .flag_if_supported("-std=c++11")
        // Silences a lot of warnings on macOS
        .flag_if_supported("-Wno-deprecated-declarations")
        // This allows us to treat rubbl's modified casacore as a separate
        // namespace, so that both vanilla casacore and rubbl can be linked
        // at the same time.
        .define("casacore", "rubbl_casacore")
        // Without this, using casa in multiple threads causes segfaults
        .define("USE_THREADS", "1")
        .include(".")
        .files(FILES)
        .compile("libcasatables_impl.a");

    for file in FILES {
        println!("cargo:rerun-if-changed={file}");
    }

    // Install the C++ headers into the output directory so that dependent
    // packages (namely, rubbl_casatables) can use them. This is modeled off of
    // how libz-sys does things. We need to have a `links =` key in the
    // Cargo.toml for this output to be respected; we say `links = "casa"` which
    // means that the `cargo:include` setting is exposed as the
    // `DEP_CASA_INCLUDE` environment variable.

    let dst = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    for header in HEADERS {
        let hdest = dst.join("include").join(header);
        fs::create_dir_all(hdest.parent().unwrap()).unwrap();
        fs::copy(header, hdest).unwrap();
    }

    println!("cargo:root={}", dst.to_str().unwrap());
    println!("cargo:include={}/include", dst.to_str().unwrap());
}

const FILES: &[&str] = &[
    "casacore/casa/Arrays/Array2.cc",
    "casacore/casa/Arrays/Array2Math.cc",
    "casacore/casa/Arrays/ArrayBase.cc",
    "casacore/casa/Arrays/ArrayError.cc",
    "casacore/casa/Arrays/ArrayOpsDiffShapes.cc",
    "casacore/casa/Arrays/ArrayPartMath.cc",
    "casacore/casa/Arrays/ArrayPosIter.cc",
    "casacore/casa/Arrays/Array_tmpl.cc",
    "casacore/casa/Arrays/ArrayUtil2.cc",
    "casacore/casa/Arrays/AxesMapping.cc",
    "casacore/casa/Arrays/AxesSpecifier.cc",
    "casacore/casa/Arrays/ExtendSpecifier.cc",
    "casacore/casa/IO/IPositionIO.cc",
    "casacore/casa/Arrays/IPosition.cc",
    "casacore/casa/Arrays/MaskArrMath2.cc",
    "casacore/casa/Arrays/Matrix2Math.cc",
    "casacore/casa/Arrays/Matrix_tmpl.cc",
    "casacore/casa/Arrays/Slice.cc",
    "casacore/casa/Arrays/Slicer.cc",
    "casacore/casa/Arrays/Vector_tmpl.cc",
    "casacore/casa/BasicMath/Math.cc",
    "casacore/casa/BasicMath/Primes.cc",
    "casacore/casa/BasicMath/Random.cc",
    "casacore/casa/BasicSL/Complex.cc",
    "casacore/casa/BasicSL/Constants.cc",
    "casacore/casa/BasicSL/IComplex.cc",
    "casacore/casa/BasicSL/STLMath.cc",
    "casacore/casa/BasicSL/String.cc",
    "casacore/casa/Containers/Allocator.cc",
    "casacore/casa/Containers/Block.cc",
    "casacore/casa/Containers/Block_tmpl.cc",
    "casacore/casa/Containers/IterError.cc",
    "casacore/casa/Containers/Record2.cc",
    "casacore/casa/Containers/Record2Interface.cc",
    "casacore/casa/Containers/Record.cc",
    "casacore/casa/Containers/RecordDesc.cc",
    "casacore/casa/Containers/RecordDescRep.cc",
    "casacore/casa/Containers/RecordField2Writer.cc",
    "casacore/casa/Containers/RecordFieldId.cc",
    "casacore/casa/Containers/RecordInterface.cc",
    "casacore/casa/Containers/RecordRep.cc",
    "casacore/casa/Containers/ValueHolder.cc",
    "casacore/casa/Containers/ValueHolderRep.cc",
    "casacore/casa/Exceptions/CasaErrorTools.cc",
    "casacore/casa/Exceptions/Error2.cc",
    "casacore/casa/HDF5/HDF5DataSet.cc",
    "casacore/casa/HDF5/HDF5DataType.cc",
    "casacore/casa/HDF5/HDF5Error.cc",
    "casacore/casa/HDF5/HDF5File.cc",
    "casacore/casa/HDF5/HDF5Group.cc",
    "casacore/casa/HDF5/HDF5HidMeta.cc",
    "casacore/casa/HDF5/HDF5Object.cc",
    "casacore/casa/HDF5/HDF5Record.cc",
    "casacore/casa/IO/AipsIO.cc",
    "casacore/casa/IO/BaseSinkSource.cc",
    "casacore/casa/IO/BucketBase.cc",
    "casacore/casa/IO/BucketBuffered.cc",
    "casacore/casa/IO/BucketCache.cc",
    "casacore/casa/IO/BucketFile.cc",
    "casacore/casa/IO/BucketMapped.cc",
    "casacore/casa/IO/ByteIO.cc",
    "casacore/casa/IO/ByteSink.cc",
    "casacore/casa/IO/ByteSinkSource.cc",
    "casacore/casa/IO/ByteSource.cc",
    "casacore/casa/IO/CanonicalIO.cc",
    "casacore/casa/IO/ConversionIO.cc",
    "casacore/casa/IO/FilebufIO.cc",
    "casacore/casa/IO/FiledesIO.cc",
    "casacore/casa/IO/FileLocker.cc",
    "casacore/casa/IO/LECanonicalIO.cc",
    "casacore/casa/IO/LockFile.cc",
    "casacore/casa/IO/MemoryIO.cc",
    "casacore/casa/IO/MFFileIO.cc",
    "casacore/casa/IO/MMapfdIO.cc",
    "casacore/casa/IO/MMapIO.cc",
    "casacore/casa/IO/MultiFileBase.cc",
    "casacore/casa/IO/MultiFile.cc",
    "casacore/casa/IO/MultiHDF5.cc",
    "casacore/casa/IO/RawIO.cc",
    "casacore/casa/IO/RegularFileIO.cc",
    "casacore/casa/IO/StreamIO.cc",
    "casacore/casa/IO/TapeIO.cc",
    "casacore/casa/IO/TypeIO.cc",
    "casacore/casa/Logging/LogFilter.cc",
    "casacore/casa/Logging/LogFilterInterface.cc",
    "casacore/casa/Logging/LogIO.cc",
    "casacore/casa/Logging/LogMessage.cc",
    "casacore/casa/Logging/LogOrigin.cc",
    "casacore/casa/Logging/LogSink.cc",
    "casacore/casa/Logging/LogSinkInterface.cc",
    "casacore/casa/Logging/MemoryLogSink.cc",
    "casacore/casa/Logging/NullLogSink.cc",
    "casacore/casa/Logging/StreamLogSink.cc",
    "casacore/casa/OS/CanonicalConversion.cc",
    "casacore/casa/OS/CanonicalDataConversion.cc",
    "casacore/casa/OS/Conversion.cc",
    "casacore/casa/OS/DataConversion.cc",
    "casacore/casa/OS/Directory.cc",
    "casacore/casa/OS/DirectoryIterator.cc",
    "casacore/casa/OS/DOos.cc",
    "casacore/casa/OS/DynLib.cc",
    "casacore/casa/OS/EnvVar.cc",
    "casacore/casa/OS/File.cc",
    "casacore/casa/OS/HostInfo.cc",
    "casacore/casa/OS/IBMConversion.cc",
    "casacore/casa/OS/IBMDataConversion.cc",
    "casacore/casa/OS/LECanonicalConversion.cc",
    "casacore/casa/OS/LECanonicalDataConversion.cc",
    "casacore/casa/OS/LittleEndianConversion.cc",
    "casacore/casa/OS/malloc.cc",
    "casacore/casa/OS/Memory.cc",
    "casacore/casa/OS/MemoryTrace.cc",
    "casacore/casa/OS/ModcompConversion.cc",
    "casacore/casa/OS/ModcompDataConversion.cc",
    "casacore/casa/OS/Path.cc",
    "casacore/casa/OS/PrecTimer.cc",
    "casacore/casa/OS/RawDataConversion.cc",
    "casacore/casa/OS/RegularFile.cc",
    "casacore/casa/OS/SymLink.cc",
    "casacore/casa/OS/Time.cc",
    "casacore/casa/OS/Timer.cc",
    "casacore/casa/OS/VAXConversion.cc",
    "casacore/casa/OS/VAXDataConversion.cc",
    "casacore/casa/Quanta/Euler.cc",
    "casacore/casa/Quanta/MeasValue.cc",
    "casacore/casa/Quanta/MVAngle.cc",
    "casacore/casa/Quanta/MVBaseline.cc",
    "casacore/casa/Quanta/MVDirection.cc",
    "casacore/casa/Quanta/MVDoppler.cc",
    "casacore/casa/Quanta/MVDouble.cc",
    "casacore/casa/Quanta/MVEarthMagnetic.cc",
    "casacore/casa/Quanta/MVEpoch.cc",
    "casacore/casa/Quanta/MVFrequency.cc",
    "casacore/casa/Quanta/MVPosition.cc",
    "casacore/casa/Quanta/MVRadialVelocity.cc",
    "casacore/casa/Quanta/MVTime.cc",
    "casacore/casa/Quanta/MVuvw.cc",
    "casacore/casa/Quanta/QBase.cc",
    "casacore/casa/Quanta/QC.cc",
    "casacore/casa/Quanta/QLogical2.cc",
    "casacore/casa/Quanta/QMath2.cc",
    "casacore/casa/Quanta/Quantum2.cc",
    "casacore/casa/Quanta/QuantumHolder.cc",
    "casacore/casa/Quanta/RotMatrix.cc",
    "casacore/casa/Quanta/Unit.cc",
    "casacore/casa/Quanta/UnitDim.cc",
    "casacore/casa/Quanta/UnitMap2.cc",
    "casacore/casa/Quanta/UnitMap3.cc",
    "casacore/casa/Quanta/UnitMap4.cc",
    "casacore/casa/Quanta/UnitMap5.cc",
    "casacore/casa/Quanta/UnitMap6.cc",
    "casacore/casa/Quanta/UnitMap7.cc",
    "casacore/casa/Quanta/UnitMap.cc",
    "casacore/casa/Quanta/UnitName.cc",
    "casacore/casa/Quanta/UnitVal.cc",
    "casacore/casa/System/AipsrcBool.cc",
    "casacore/casa/System/Aipsrc.cc",
    "casacore/casa/System/AipsrcValue2.cc",
    "casacore/casa/System/AipsrcVBool.cc",
    "casacore/casa/System/AipsrcVString.cc",
    "casacore/casa/System/AppInfo.cc",
    "casacore/casa/System/Casarc.cc",
    "casacore/casa/System/Choice.cc",
    "casacore/casa/System/ObjectID2.cc",
    "casacore/casa/System/ObjectID.cc",
    "casacore/casa/System/PGPlotter.cc",
    "casacore/casa/System/PGPlotterInterface.cc",
    "casacore/casa/System/PGPlotterNull.cc",
    "casacore/casa/System/ProgressMeter.cc",
    "casacore/casa/Utilities/AlignMemory.cc",
    "casacore/casa/Utilities/BitVector.cc",
    "casacore/casa/Utilities/Compare.cc",
    "casacore/casa/Utilities/CompositeNumber.cc",
    "casacore/casa/Utilities/Copy2.cc",
    "casacore/casa/Utilities/CountedPtr2.cc",
    "casacore/casa/Utilities/DataType.cc",
    "casacore/casa/Utilities/DynBuffer.cc",
    "casacore/casa/Utilities/Fallible2.cc",
    "casacore/casa/Utilities/MUString.cc",
    "casacore/casa/Utilities/Notice.cc",
    "casacore/casa/Utilities/Precision.cc",
    "casacore/casa/Utilities/RecordTransformable.cc",
    "casacore/casa/Utilities/Regex.cc",
    "casacore/casa/Utilities/Sequence2.cc",
    "casacore/casa/Utilities/Sort.cc",
    "casacore/casa/Utilities/SortError.cc",
    "casacore/casa/Utilities/StringDistance.cc",
    "casacore/casa/Utilities/ValType.cc",
    "casacore/tables/DataMan/BitFlagsEngine.cc",
    "casacore/tables/DataMan/CompressComplex.cc",
    "casacore/tables/DataMan/CompressFloat.cc",
    "casacore/tables/DataMan/DataManAccessor.cc",
    "casacore/tables/DataMan/DataManager.cc",
    "casacore/tables/DataMan/DataManagerColumn.cc",
    "casacore/tables/DataMan/DataManError.cc",
    "casacore/tables/DataMan/DataManInfo.cc",
    "casacore/tables/DataMan/ForwardCol.cc",
    "casacore/tables/DataMan/ForwardColRow.cc",
    "casacore/tables/DataMan/IncrementalStMan.cc",
    "casacore/tables/DataMan/IncrStManAccessor.cc",
    "casacore/tables/DataMan/ISMBase.cc",
    "casacore/tables/DataMan/ISMBucket.cc",
    "casacore/tables/DataMan/ISMColumn.cc",
    "casacore/tables/DataMan/ISMIndColumn.cc",
    "casacore/tables/DataMan/ISMIndex.cc",
    "casacore/tables/DataMan/MemoryStMan.cc",
    "casacore/tables/DataMan/MSMBase.cc",
    "casacore/tables/DataMan/MSMColumn.cc",
    "casacore/tables/DataMan/MSMDirColumn.cc",
    "casacore/tables/DataMan/MSMIndColumn.cc",
    "casacore/tables/DataMan/SSMBase.cc",
    "casacore/tables/DataMan/SSMColumn.cc",
    "casacore/tables/DataMan/SSMDirColumn.cc",
    "casacore/tables/DataMan/SSMIndColumn.cc",
    "casacore/tables/DataMan/SSMIndex.cc",
    "casacore/tables/DataMan/SSMIndStringColumn.cc",
    "casacore/tables/DataMan/SSMStringHandler.cc",
    "casacore/tables/DataMan/StandardStManAccessor.cc",
    "casacore/tables/DataMan/StandardStMan.cc",
    "casacore/tables/DataMan/StArrAipsIO.cc",
    "casacore/tables/DataMan/StArrayFile.cc",
    "casacore/tables/DataMan/StIndArrAIO.cc",
    "casacore/tables/DataMan/StIndArray.cc",
    "casacore/tables/DataMan/StManAipsIO.cc",
    "casacore/tables/DataMan/StManColumn.cc",
    "casacore/tables/DataMan/StManColumnBase.cc",
    "casacore/tables/DataMan/TiledCellStMan.cc",
    "casacore/tables/DataMan/TiledColumnStMan.cc",
    "casacore/tables/DataMan/TiledDataStManAccessor.cc",
    "casacore/tables/DataMan/TiledDataStMan.cc",
    "casacore/tables/DataMan/TiledFileAccess.cc",
    "casacore/tables/DataMan/TiledFileHelper.cc",
    "casacore/tables/DataMan/TiledShapeStMan.cc",
    "casacore/tables/DataMan/TiledStManAccessor.cc",
    "casacore/tables/DataMan/TiledStMan.cc",
    "casacore/tables/DataMan/TSMColumn.cc",
    "casacore/tables/DataMan/TSMCoordColumn.cc",
    "casacore/tables/DataMan/TSMCubeBuff.cc",
    "casacore/tables/DataMan/TSMCube.cc",
    "casacore/tables/DataMan/TSMCubeMMap.cc",
    "casacore/tables/DataMan/TSMDataColumn.cc",
    "casacore/tables/DataMan/TSMFile.cc",
    "casacore/tables/DataMan/TSMIdColumn.cc",
    "casacore/tables/DataMan/TSMOption.cc",
    "casacore/tables/DataMan/TSMShape.cc",
    "casacore/tables/DataMan/VirtColEng.cc",
    "casacore/tables/DataMan/VirtScaCol.cc",
    "casacore/tables/DataMan/VirtArrCol.cc",
    "casacore/tables/Tables/ArrayColumn_tmpl.cc",
    "casacore/tables/Tables/ArrColDesc_tmpl.cc",
    "casacore/tables/Tables/ArrayColumnBase.cc",
    "casacore/tables/Tables/ArrColDesc.cc",
    "casacore/tables/Tables/ArrColData.cc",
    "casacore/tables/Tables/BaseColDesc.cc",
    "casacore/tables/Tables/BaseColumn.cc",
    "casacore/tables/Tables/BaseTabIter.cc",
    "casacore/tables/Tables/BaseTable.cc",
    "casacore/tables/Tables/ColDescSet.cc",
    "casacore/tables/Tables/ColumnCache.cc",
    "casacore/tables/Tables/ColumnDesc.cc",
    "casacore/tables/Tables/ColumnSet.cc",
    "casacore/tables/Tables/ColumnsIndexArray.cc",
    "casacore/tables/Tables/ColumnsIndex.cc",
    "casacore/tables/Tables/ConcatColumn.cc",
    "casacore/tables/Tables/ConcatRows.cc",
    "casacore/tables/Tables/ConcatTable.cc",
    "casacore/tables/Tables/ExternalLockSync.cc",
    "casacore/tables/Tables/MemoryTable.cc",
    "casacore/tables/Tables/NullTable.cc",
    "casacore/tables/Tables/PlainColumn.cc",
    "casacore/tables/Tables/PlainTable.cc",
    "casacore/tables/Tables/ReadAsciiTable.cc",
    "casacore/tables/Tables/RefColumn.cc",
    "casacore/tables/Tables/RefRows.cc",
    "casacore/tables/Tables/RefTable.cc",
    "casacore/tables/Tables/RowCopier.cc",
    "casacore/tables/Tables/RowNumbers.cc",
    "casacore/tables/Tables/ScaColDesc_tmpl.cc",
    "casacore/tables/Tables/ScalarColumn_tmpl.cc",
    "casacore/tables/Tables/ScaRecordColData.cc",
    "casacore/tables/Tables/ScaRecordColDesc.cc",
    "casacore/tables/Tables/SetupNewTab.cc",
    "casacore/tables/Tables/StorageOption.cc",
    "casacore/tables/Tables/SubTabDesc.cc",
    "casacore/tables/Tables/TableAttr.cc",
    "casacore/tables/Tables/TableCache.cc",
    "casacore/tables/Tables/Table.cc",
    "casacore/tables/Tables/TableColumn.cc",
    "casacore/tables/Tables/TableCopy.cc",
    "casacore/tables/Tables/TableDesc.cc",
    "casacore/tables/Tables/TableError.cc",
    "casacore/tables/Tables/TableInfo.cc",
    "casacore/tables/Tables/TableIter.cc",
    "casacore/tables/Tables/TableKeyword.cc",
    "casacore/tables/Tables/TableLock.cc",
    "casacore/tables/Tables/TableLockData.cc",
    "casacore/tables/Tables/TableLocker.cc",
    "casacore/tables/Tables/TableRecord.cc",
    "casacore/tables/Tables/TableRecordRep.cc",
    "casacore/tables/Tables/TableRow.cc",
    "casacore/tables/Tables/TableSyncData.cc",
    "casacore/tables/Tables/TableTrace.cc",
    "casacore/tables/Tables/TabPath.cc",
];

const HEADERS: &[&str] = &[
    "casacore/casa/aipsdef.h",
    "casacore/casa/aipsenv.h",
    "casacore/casa/aips.h",
    "casacore/casa/aipstype.h",
    "casacore/casa/aipsxtype.h",
    "casacore/casa/Arrays/ArrayAccessor.h",
    "casacore/casa/Arrays/ArrayBase.h",
    "casacore/casa/Arrays/ArrayError.h",
    "casacore/casa/Arrays/Array.h",
    "casacore/casa/IO/ArrayIO.h",
    "casacore/casa/IO/ArrayIO.tcc",
    "casacore/casa/Arrays/ArrayIter.h",
    "casacore/casa/Arrays/ArrayIter.tcc",
    "casacore/casa/Arrays/ArrayLogical.h",
    "casacore/casa/Arrays/ArrayLogical.tcc",
    "casacore/casa/Arrays/ArrayMathBase.h",
    "casacore/casa/Arrays/ArrayMath.h",
    "casacore/casa/Arrays/ArrayMath.tcc",
    "casacore/casa/Arrays/ArrayOpsDiffShapes.h",
    "casacore/casa/Arrays/ArrayOpsDiffShapes.tcc",
    "casacore/casa/Arrays/ArrayPartMath.h",
    "casacore/casa/Arrays/ArrayPartMath.tcc",
    "casacore/casa/Arrays/ArrayPosIter.h",
    "casacore/casa/Arrays/ArrayStr.tcc",
    "casacore/casa/Arrays/ArrayStr.h",
    "casacore/casa/Arrays/Array.tcc",
    "casacore/casa/Arrays/ArrayUtil.h",
    "casacore/casa/Arrays/ArrayUtil.tcc",
    "casacore/casa/Arrays/AxesMapping.h",
    "casacore/casa/Arrays/AxesSpecifier.h",
    "casacore/casa/Arrays/Cube.h",
    "casacore/casa/Arrays/Cube.tcc",
    "casacore/casa/Arrays/ElementFunctions.h",
    "casacore/casa/Arrays/ExtendSpecifier.h",
    "casacore/casa/Arrays.h",
    "casacore/casa/Arrays/IPosition.h",
    "casacore/casa/Arrays/ArrayFwd.h",
    "casacore/casa/Arrays/LogiArray.h",
    "casacore/casa/Arrays/LogiCube.h",
    "casacore/casa/Arrays/LogiMatrix.h",
    "casacore/casa/Arrays/LogiVector.h",
    "casacore/casa/Arrays/MaskArrIO.h",
    "casacore/casa/Arrays/MaskArrIO.tcc",
    "casacore/casa/Arrays/MaskArrLogi.h",
    "casacore/casa/Arrays/MaskArrLogi.tcc",
    "casacore/casa/Arrays/MaskArrMath.h",
    "casacore/casa/Arrays/MaskArrMath.tcc",
    "casacore/casa/Arrays/MaskedArray.h",
    "casacore/casa/Arrays/MaskedArray.tcc",
    "casacore/casa/Arrays/MaskLogiArrFwd.h",
    "casacore/casa/Arrays/MaskLogiArr.h",
    "casacore/casa/Arrays/Matrix.h",
    "casacore/casa/Arrays/MatrixIter.h",
    "casacore/casa/Arrays/MatrixIter.tcc",
    "casacore/casa/Arrays/MatrixMath.h",
    "casacore/casa/Arrays/MatrixMath.tcc",
    "casacore/casa/Arrays/Matrix.tcc",
    "casacore/casa/Arrays/Memory.h",
    "casacore/casa/Arrays/Slice.h",
    "casacore/casa/Arrays/Slicer.h",
    "casacore/casa/Arrays/Storage.h",
    "casacore/casa/Arrays/Vector2.tcc",
    "casacore/casa/Arrays/Vector.h",
    "casacore/casa/Arrays/VectorIter.h",
    "casacore/casa/Arrays/VectorIter.tcc",
    "casacore/casa/Arrays/VectorSTLIterator.h",
    "casacore/casa/Arrays/Vector.tcc",
    "casacore/casa/BasicMath/ConvertScalar.h",
    "casacore/casa/BasicMath/Functional.h",
    "casacore/casa/BasicMath/Functional.tcc",
    "casacore/casa/BasicMath/Functors.h",
    "casacore/casa/BasicMath.h",
    "casacore/casa/BasicMath/Math.h",
    "casacore/casa/BasicMath/Primes.h",
    "casacore/casa/BasicMath/Random.h",
    "casacore/casa/BasicMath/StdLogical.h",
    "casacore/casa/BasicSL/Complexfwd.h",
    "casacore/casa/BasicSL/Complex.h",
    "casacore/casa/BasicSL/Constants.h",
    "casacore/casa/BasicSL.h",
    "casacore/casa/BasicSL/IComplex.h",
    "casacore/casa/BasicSL/STLIO.h",
    "casacore/casa/BasicSL/STLIO.tcc",
    "casacore/casa/BasicSL/STLMath.h",
    "casacore/casa/BasicSL/String.h",
    "casacore/casa/complex.h",
    "casacore/casa/config.h",
    "casacore/casa/Containers/Allocator.h",
    "casacore/casa/Containers/Block.h",
    "casacore/casa/Containers/BlockIO.h",
    "casacore/casa/Containers/BlockIO.tcc",
    "casacore/casa/Containers.h",
    "casacore/casa/Containers/IterError.h",
    "casacore/casa/Containers/Link.h",
    "casacore/casa/Containers/Link.tcc",
    "casacore/casa/Containers/ObjectStack.h",
    "casacore/casa/Containers/ObjectStack.tcc",
    "casacore/casa/Containers/RecordDesc.h",
    "casacore/casa/Containers/RecordDescRep.h",
    "casacore/casa/Containers/RecordField.h",
    "casacore/casa/Containers/RecordFieldId.h",
    "casacore/casa/Containers/RecordField.tcc",
    "casacore/casa/Containers/RecordFieldWriter.h",
    "casacore/casa/Containers/RecordFieldWriter.tcc",
    "casacore/casa/Containers/Record.h",
    "casacore/casa/Containers/RecordInterface.h",
    "casacore/casa/Containers/RecordRep.h",
    "casacore/casa/Containers/ValueHolder.h",
    "casacore/casa/Containers/ValueHolderRep.h",
    "casacore/casa/Exceptions/CasaErrorTools.h",
    "casacore/casa/Exceptions/Error.h",
    "casacore/casa/Exceptions/Error.tcc",
    "casacore/casa/Exceptions.h",
    "casacore/casa/fstream.h",
    "casacore/casa/HDF5.h",
    "casacore/casa/HDF5/HDF5DataSet.h",
    "casacore/casa/HDF5/HDF5DataType.h",
    "casacore/casa/HDF5/HDF5Error.h",
    "casacore/casa/HDF5/HDF5File.h",
    "casacore/casa/HDF5/HDF5Group.h",
    "casacore/casa/HDF5/HDF5HidMeta.h",
    "casacore/casa/HDF5/HDF5Object.h",
    "casacore/casa/HDF5/HDF5Record.h",
    "casacore/casa/Inputs.h",
    "casacore/casa/IO/AipsIOCarray.h",
    "casacore/casa/IO/AipsIOCarray.tcc",
    "casacore/casa/IO/AipsIO.h",
    "casacore/casa/IO/BaseSinkSource.h",
    "casacore/casa/IO/BucketBase.h",
    "casacore/casa/IO/BucketBuffered.h",
    "casacore/casa/IO/BucketCache.h",
    "casacore/casa/IO/BucketFile.h",
    "casacore/casa/IO/BucketMapped.h",
    "casacore/casa/IO/ByteIO.h",
    "casacore/casa/IO/ByteSink.h",
    "casacore/casa/IO/ByteSinkSource.h",
    "casacore/casa/IO/ByteSource.h",
    "casacore/casa/IO/CanonicalIO.h",
    "casacore/casa/IO/ConversionIO.h",
    "casacore/casa/IO/FilebufIO.h",
    "casacore/casa/IO/FiledesIO.h",
    "casacore/casa/IO/FileLocker.h",
    "casacore/casa/IO.h",
    "casacore/casa/IO/LargeIOFuncDef.h",
    "casacore/casa/IO/LECanonicalIO.h",
    "casacore/casa/IO/LockFile.h",
    "casacore/casa/iomanip.h",
    "casacore/casa/IO/MemoryIO.h",
    "casacore/casa/IO/MFFileIO.h",
    "casacore/casa/IO/MMapfdIO.h",
    "casacore/casa/IO/MMapIO.h",
    "casacore/casa/IO/MultiFileBase.h",
    "casacore/casa/IO/MultiFile.h",
    "casacore/casa/IO/MultiHDF5.h",
    "casacore/casa/IO/RawIO.h",
    "casacore/casa/IO/RegularFileIO.h",
    "casacore/casa/iosfwd.h",
    "casacore/casa/iosstrfwd.h",
    "casacore/casa/iostream.h",
    "casacore/casa/IO/StreamIO.h",
    "casacore/casa/IO/TapeIO.h",
    "casacore/casa/IO/TypeIO.h",
    "casacore/casa/istream.h",
    "casacore/casa/Json.h",
    "casacore/casa/Logging.h",
    "casacore/casa/Logging/LogFilter.h",
    "casacore/casa/Logging/LogFilterInterface.h",
    "casacore/casa/Logging/LogIO.h",
    "casacore/casa/Logging/LogMessage.h",
    "casacore/casa/Logging/LogOrigin.h",
    "casacore/casa/Logging/LogSink.h",
    "casacore/casa/Logging/LogSinkInterface.h",
    "casacore/casa/Logging/MemoryLogSink.h",
    "casacore/casa/Logging/NullLogSink.h",
    "casacore/casa/Logging/StreamLogSink.h",
    "casacore/casa/math.h",
    "casacore/casa/namespace.h",
    "casacore/casa/OS/CanonicalConversion.h",
    "casacore/casa/OS/CanonicalDataConversion.h",
    "casacore/casa/OS/Conversion.h",
    "casacore/casa/OS/DataConversion.h",
    "casacore/casa/OS/Directory.h",
    "casacore/casa/OS/DirectoryIterator.h",
    "casacore/casa/OS/DOos.h",
    "casacore/casa/OS/DynLib.h",
    "casacore/casa/OS/EnvVar.h",
    "casacore/casa/OS/File.h",
    "casacore/casa/OS.h",
    "casacore/casa/OS/HostInfoBsd.h",
    "casacore/casa/OS/HostInfoDarwin.h",
    "casacore/casa/OS/HostInfo.h",
    "casacore/casa/OS/HostInfoHpux.h",
    "casacore/casa/OS/HostInfoIrix.h",
    "casacore/casa/OS/HostInfoLinux.h",
    "casacore/casa/OS/HostInfoOsf1.h",
    "casacore/casa/OS/HostInfoSolaris.h",
    "casacore/casa/OS/IBMConversion.h",
    "casacore/casa/OS/IBMDataConversion.h",
    "casacore/casa/OS/LECanonicalConversion.h",
    "casacore/casa/OS/LECanonicalDataConversion.h",
    "casacore/casa/OS/LittleEndianConversion.h",
    "casacore/casa/OS/malloc.h",
    "casacore/casa/OS/Memory.h",
    "casacore/casa/OS/MemoryTrace.h",
    "casacore/casa/OS/ModcompConversion.h",
    "casacore/casa/OS/ModcompDataConversion.h",
    "casacore/casa/OS/Mutex.h",
    "casacore/casa/OS/Path.h",
    "casacore/casa/OS/PrecTimer.h",
    "casacore/casa/OS/RawDataConversion.h",
    "casacore/casa/OS/RegularFile.h",
    "casacore/casa/OS/SymLink.h",
    "casacore/casa/OS/Time.h",
    "casacore/casa/OS/Timer.h",
    "casacore/casa/ostream.h",
    "casacore/casa/OS/VAXConversion.h",
    "casacore/casa/OS/VAXDataConversion.h",
    "casacore/casa/Quanta/Euler.h",
    "casacore/casa/Quanta.h",
    "casacore/casa/Quanta/MeasValue.h",
    "casacore/casa/Quanta/MVAngle.h",
    "casacore/casa/Quanta/MVBaseline.h",
    "casacore/casa/Quanta/MVDirection.h",
    "casacore/casa/Quanta/MVDoppler.h",
    "casacore/casa/Quanta/MVDouble.h",
    "casacore/casa/Quanta/MVEarthMagnetic.h",
    "casacore/casa/Quanta/MVEpoch.h",
    "casacore/casa/Quanta/MVFrequency.h",
    "casacore/casa/Quanta/MVPosition.h",
    "casacore/casa/Quanta/MVRadialVelocity.h",
    "casacore/casa/Quanta/MVTime.h",
    "casacore/casa/Quanta/MVuvw.h",
    "casacore/casa/Quanta/QBase.h",
    "casacore/casa/Quanta/QC.h",
    "casacore/casa/Quanta/QLogical.h",
    "casacore/casa/Quanta/QLogical.tcc",
    "casacore/casa/Quanta/QMath.h",
    "casacore/casa/Quanta/QMath.tcc",
    "casacore/casa/Quanta/Quantum.h",
    "casacore/casa/Quanta/QuantumHolder.h",
    "casacore/casa/Quanta/Quantum.tcc",
    "casacore/casa/Quanta/QuantumType.h",
    "casacore/casa/Quanta/QVector.h",
    "casacore/casa/Quanta/QVector.tcc",
    "casacore/casa/Quanta/RotMatrix.h",
    "casacore/casa/Quanta/UnitDim.h",
    "casacore/casa/Quanta/Unit.h",
    "casacore/casa/Quanta/UnitMap.h",
    "casacore/casa/Quanta/UnitName.h",
    "casacore/casa/Quanta/UnitVal.h",
    "casacore/casa/sstream.h",
    "casacore/casa/stdexcept.h",
    "casacore/casa/stdio.h",
    "casacore/casa/stdlib.h",
    "casacore/casa/stdmap.h",
    "casacore/casa/stdvector.h",
    "casacore/casa/string.h",
    "casacore/casa/System/Aipsrc.h",
    "casacore/casa/System/AipsrcValue.h",
    "casacore/casa/System/AipsrcValue.tcc",
    "casacore/casa/System/AipsrcVector.h",
    "casacore/casa/System/AipsrcVector.tcc",
    "casacore/casa/System/AppInfo.h",
    "casacore/casa/System/Casarc.h",
    "casacore/casa/System/Choice.h",
    "casacore/casa/System.h",
    "casacore/casa/System/ObjectID.h",
    "casacore/casa/System/PGPlotter.h",
    "casacore/casa/System/PGPlotterInterface.h",
    "casacore/casa/System/PGPlotterNull.h",
    "casacore/casa/System/ProgressMeter.h",
    "casacore/casa/typeinfo.h",
    "casacore/casa/Utilities/AlignMemory.h",
    "casacore/casa/Utilities/Assert.h",
    "casacore/casa/Utilities/Assert.tcc",
    "casacore/casa/Utilities/BinarySearch.h",
    "casacore/casa/Utilities/BinarySearch.tcc",
    "casacore/casa/Utilities/BitVector.h",
    "casacore/casa/Utilities/CASATask.h",
    "casacore/casa/Utilities/Compare.h",
    "casacore/casa/Utilities/Compare.tcc",
    "casacore/casa/Utilities/CompositeNumber.h",
    "casacore/casa/Utilities/Copy.h",
    "casacore/casa/Utilities/Copy.tcc",
    "casacore/casa/Utilities/CountedPtr.h",
    "casacore/casa/Utilities/CountedPtr.tcc",
    "casacore/casa/Utilities/COWPtr.h",
    "casacore/casa/Utilities/COWPtr.tcc",
    "casacore/casa/Utilities/DataType.h",
    "casacore/casa/Utilities/DefaultValue.h",
    "casacore/casa/Utilities/DynBuffer.h",
    "casacore/casa/Utilities/Fallible.h",
    "casacore/casa/Utilities/generic.h",
    "casacore/casa/Utilities/GenSort.h",
    "casacore/casa/Utilities/GenSort.tcc",
    "casacore/casa/Utilities.h",
    "casacore/casa/Utilities/LinearSearch.h",
    "casacore/casa/Utilities/LinearSearch.tcc",
    "casacore/casa/Utilities/MUString.h",
    "casacore/casa/Utilities/Notice.h",
    "casacore/casa/Utilities/Precision.h",
    "casacore/casa/Utilities/PtrHolder.h",
    "casacore/casa/Utilities/PtrHolder.tcc",
    "casacore/casa/Utilities/RecordTransformable.h",
    "casacore/casa/Utilities/Regex.h",
    "casacore/casa/Utilities/Sequence.h",
    "casacore/casa/Utilities/Sequence.tcc",
    "casacore/casa/Utilities/SortError.h",
    "casacore/casa/Utilities/Sort.h",
    "casacore/casa/Utilities/Sort.tcc",
    "casacore/casa/Utilities/StringDistance.h",
    "casacore/casa/Utilities/Template.h",
    "casacore/casa/Utilities/Template.tcc",
    "casacore/casa/Utilities/ValType.h",
    "casacore/casa/Utilities/ValTypeId.h",
    "casacore/casa/vector.h",
    "casacore/casa/version.h",
    "casacore/tables/DataMan/BaseMappedArrayEngine.h",
    "casacore/tables/DataMan/BaseMappedArrayEngine.tcc",
    "casacore/tables/DataMan/BitFlagsEngine.h",
    "casacore/tables/DataMan/BitFlagsEngine.tcc",
    "casacore/tables/DataMan/CompressComplex.h",
    "casacore/tables/DataMan/CompressFloat.h",
    "casacore/tables/DataMan/DataManAccessor.h",
    "casacore/tables/DataMan/DataManager.h",
    "casacore/tables/DataMan/DataManagerColumn.h",
    "casacore/tables/DataMan/DataManError.h",
    "casacore/tables/DataMan/DataManInfo.h",
    "casacore/tables/DataMan/ForwardCol.h",
    "casacore/tables/DataMan/ForwardColRow.h",
    "casacore/tables/DataMan.h",
    "casacore/tables/DataMan/IncrementalStMan.h",
    "casacore/tables/DataMan/IncrStManAccessor.h",
    "casacore/tables/DataMan/ISMBase.h",
    "casacore/tables/DataMan/ISMBucket.h",
    "casacore/tables/DataMan/ISMColumn.h",
    "casacore/tables/DataMan/ISMIndColumn.h",
    "casacore/tables/DataMan/ISMIndex.h",
    "casacore/tables/DataMan/MappedArrayEngine.h",
    "casacore/tables/DataMan/MappedArrayEngine.tcc",
    "casacore/tables/DataMan/MemoryStMan.h",
    "casacore/tables/DataMan/MSMBase.h",
    "casacore/tables/DataMan/MSMColumn.h",
    "casacore/tables/DataMan/MSMDirColumn.h",
    "casacore/tables/DataMan/MSMIndColumn.h",
    "casacore/tables/DataMan/RetypedArrayEngine.h",
    "casacore/tables/DataMan/RetypedArrayEngine.tcc",
    "casacore/tables/DataMan/RetypedArraySetGet.h",
    "casacore/tables/DataMan/RetypedArraySetGet.tcc",
    "casacore/tables/DataMan/ScaledArrayEngine.h",
    "casacore/tables/DataMan/ScaledArrayEngine.tcc",
    "casacore/tables/DataMan/ScaledComplexData.h",
    "casacore/tables/DataMan/ScaledComplexData.tcc",
    "casacore/tables/DataMan/SSMBase.h",
    "casacore/tables/DataMan/SSMColumn.h",
    "casacore/tables/DataMan/SSMDirColumn.h",
    "casacore/tables/DataMan/SSMIndColumn.h",
    "casacore/tables/DataMan/SSMIndex.h",
    "casacore/tables/DataMan/SSMIndStringColumn.h",
    "casacore/tables/DataMan/SSMStringHandler.h",
    "casacore/tables/DataMan/StandardStManAccessor.h",
    "casacore/tables/DataMan/StandardStMan.h",
    "casacore/tables/DataMan/StArrAipsIO.h",
    "casacore/tables/DataMan/StArrayFile.h",
    "casacore/tables/DataMan/StIndArrAIO.h",
    "casacore/tables/DataMan/StIndArray.h",
    "casacore/tables/DataMan/StManAipsIO.h",
    "casacore/tables/DataMan/StManColumn.h",
    "casacore/tables/DataMan/StManColumnBase.h",
    "casacore/tables/DataMan/TiledCellStMan.h",
    "casacore/tables/DataMan/TiledColumnStMan.h",
    "casacore/tables/DataMan/TiledDataStManAccessor.h",
    "casacore/tables/DataMan/TiledDataStMan.h",
    "casacore/tables/DataMan/TiledFileAccess.h",
    "casacore/tables/DataMan/TiledFileHelper.h",
    "casacore/tables/DataMan/TiledShapeStMan.h",
    "casacore/tables/DataMan/TiledStManAccessor.h",
    "casacore/tables/DataMan/TiledStMan.h",
    "casacore/tables/DataMan/TSMColumn.h",
    "casacore/tables/DataMan/TSMCoordColumn.h",
    "casacore/tables/DataMan/TSMCubeBuff.h",
    "casacore/tables/DataMan/TSMCube.h",
    "casacore/tables/DataMan/TSMCubeMMap.h",
    "casacore/tables/DataMan/TSMDataColumn.h",
    "casacore/tables/DataMan/TSMFile.h",
    "casacore/tables/DataMan/TSMIdColumn.h",
    "casacore/tables/DataMan/TSMOption.h",
    "casacore/tables/DataMan/TSMShape.h",
    "casacore/tables/DataMan/VirtArrCol.h",
    "casacore/tables/DataMan/VirtArrCol.tcc",
    "casacore/tables/DataMan/VirtColEng.h",
    "casacore/tables/DataMan/VirtScaCol.h",
    "casacore/tables/DataMan/VirtScaCol.tcc",
    "casacore/tables/DataMan/VSCEngine.h",
    "casacore/tables/DataMan/VSCEngine.tcc",
    "casacore/tables/DataMan/VACEngine.tcc",
    "casacore/tables/DataMan/VACEngine.h",
    "casacore/tables/LogTables.h",
    "casacore/tables/Tables/ArrayColumnFunc.h",
    "casacore/tables/Tables/ArrayColumn.h",
    "casacore/tables/Tables/ArrayColumnBase.h",
    "casacore/tables/Tables/ArrayColumn.tcc",
    "casacore/tables/Tables/ArrColData.h",
    "casacore/tables/Tables/ArrColData.cc",
    "casacore/tables/Tables/ArrColDesc.h",
    "casacore/tables/Tables/ArrColDesc.tcc",
    "casacore/tables/Tables/BaseColDesc.h",
    "casacore/tables/Tables/BaseColumn.h",
    "casacore/tables/Tables/BaseTabIter.h",
    "casacore/tables/Tables/BaseTable.h",
    "casacore/tables/Tables/ColDescSet.h",
    "casacore/tables/Tables/ColumnCache.h",
    "casacore/tables/Tables/ColumnDesc.h",
    "casacore/tables/Tables/ColumnSet.h",
    "casacore/tables/Tables/ColumnsIndexArray.h",
    "casacore/tables/Tables/ColumnsIndex.h",
    "casacore/tables/Tables/ConcatColumn.h",
    "casacore/tables/Tables/ConcatRows.h",
    "casacore/tables/Tables/ConcatScalarColumn.h",
    "casacore/tables/Tables/ConcatScalarColumn.tcc",
    "casacore/tables/Tables/ConcatTable.h",
    "casacore/tables/Tables/ExternalLockSync.h",
    "casacore/tables/Tables.h",
    "casacore/tables/Tables/MemoryTable.h",
    "casacore/tables/Tables/NullTable.h",
    "casacore/tables/Tables/PlainColumn.h",
    "casacore/tables/Tables/PlainTable.h",
    "casacore/tables/Tables/ReadAsciiTable.h",
    "casacore/tables/Tables/RefColumn.h",
    "casacore/tables/Tables/RefRows.h",
    "casacore/tables/Tables/RefTable.h",
    "casacore/tables/Tables/RowCopier.h",
    "casacore/tables/Tables/RowNumbers.h",
    "casacore/tables/Tables/ScaColData.h",
    "casacore/tables/Tables/ScaColData.tcc",
    "casacore/tables/Tables/ScaColDesc.h",
    "casacore/tables/Tables/ScaColDesc.tcc",
    "casacore/tables/Tables/ScalarColumn.h",
    "casacore/tables/Tables/ScalarColumn.tcc",
    "casacore/tables/Tables/ScaRecordColData.h",
    "casacore/tables/Tables/ScaRecordColDesc.h",
    "casacore/tables/Tables/SetupNewTab.h",
    "casacore/tables/Tables/StorageOption.h",
    "casacore/tables/Tables/SubTabDesc.h",
    "casacore/tables/Tables/TableAttr.h",
    "casacore/tables/Tables/TableCache.h",
    "casacore/tables/Tables/TableColumn.h",
    "casacore/tables/Tables/TableCopy.h",
    "casacore/tables/Tables/TableCopy.tcc",
    "casacore/tables/Tables/TableDesc.h",
    "casacore/tables/Tables/TableError.h",
    "casacore/tables/Tables/Table.h",
    "casacore/tables/Tables/TableInfo.h",
    "casacore/tables/Tables/TableIter.h",
    "casacore/tables/Tables/TableKeyword.h",
    "casacore/tables/Tables/TableLockData.h",
    "casacore/tables/Tables/TableLocker.h",
    "casacore/tables/Tables/TableLock.h",
    "casacore/tables/Tables/TableRecord.h",
    "casacore/tables/Tables/TableRecordRep.h",
    "casacore/tables/Tables/TableRow.h",
    "casacore/tables/Tables/TableSyncData.h",
    "casacore/tables/Tables/TableTrace.h",
    "casacore/tables/Tables/TableUtil.h",
    "casacore/tables/Tables/TableVector.h",
    "casacore/tables/Tables/TableVector.tcc",
    "casacore/tables/Tables/TabPath.h",
    "casacore/tables/Tables/TabVecLogic.h",
    "casacore/tables/Tables/TabVecLogic.tcc",
    "casacore/tables/Tables/TabVecMath.h",
    "casacore/tables/Tables/TabVecMath.tcc",
    "casacore/tables/Tables/TVec.h",
    "casacore/tables/Tables/TVecLogic.h",
    "casacore/tables/Tables/TVecLogic.tcc",
    "casacore/tables/Tables/TVecMath.h",
    "casacore/tables/Tables/TVecMath.tcc",
    "casacore/tables/Tables/TVecScaCol.h",
    "casacore/tables/Tables/TVecScaCol.tcc",
    "casacore/tables/Tables/TVec.tcc",
    "casacore/tables/Tables/TVecTemp.h",
    "casacore/tables/Tables/TVecTemp.tcc",
];

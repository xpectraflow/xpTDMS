import os
import tempfile
import xpTDMS

def test_python_bindings():
    print("Testing PyO3 Python Bindings for xpTDMS...")
    assert hasattr(xpTDMS, 'TdmsFile')
    assert hasattr(xpTDMS, 'defragment')
    assert hasattr(xpTDMS, 'write_channel_f64')
    print("✓ Module classes & functions verified")

    with tempfile.TemporaryDirectory() as tmpdir:
        file1 = os.path.join(tmpdir, "test1.tdms")
        file2 = os.path.join(tmpdir, "test2.tdms")

        # Write file
        data = [10.5, 20.25, 30.125, 40.0, 50.75]
        xpTDMS.write_channel_f64(file1, "Sensors", "Temperature", data)
        print("✓ Wrote channel via xpTDMS.write_channel_f64")

        # Read file
        tdms = xpTDMS.TdmsFile.open(file1)
        groups = tdms.group_names()
        assert groups == ["Sensors"]
        channels = tdms.channel_names("Sensors")
        assert channels == ["Temperature"]
        read_data = tdms.read_channel_f64("Sensors", "Temperature")
        assert read_data == data
        print("✓ Opened and verified read values match written values 100%")

        # Defragment file
        xpTDMS.defragment(file1, file2)
        tdms_defrag = xpTDMS.TdmsFile.open(file2)
        defrag_data = tdms_defrag.read_channel_f64("Sensors", "Temperature")
        assert defrag_data == data
        print("✓ Defragmented file successfully verified")

    print("🎉 All Python binding tests passed!")

if __name__ == "__main__":
    test_python_bindings()

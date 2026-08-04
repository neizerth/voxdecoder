from setuptools import setup, find_packages

setup(
    name="vd-text-py",
    version="0.1.0",
    description="Natasha/razdel sidecar for voxdecoder linguistic infrastructure",
    author="VoxDecoder Contributors",
    packages=find_packages(),
    python_requires=">=3.8",
    install_requires=[
        "natasha>=1.0.0",
        "razdel>=0.5.0",
        "pydantic>=2.0.0",
    ],
    entry_points={
        "console_scripts": [
            "vd-text-py=vd_text_py.main:main",
        ],
    },
    extras_require={
        "dev": [
            "pytest>=7.0.0",
            "pytest-cov>=4.0.0",
        ],
    },
)

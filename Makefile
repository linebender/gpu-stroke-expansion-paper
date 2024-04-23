paper:
	pdflatex paper & bibtex paper

clean:
	rm paper.aux & rm paper.log & rm paper.out & rm paper.bbl & rm paper.pdf

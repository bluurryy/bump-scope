inspect_asm::vec_map::try_grow:
	push r15
	push r14
	push r13
	push r12
	push rbx
	mov r14, rdi
	mov rbx, qword ptr [rsi + 24]
	mov r12, qword ptr [rsi]
	mov r15, qword ptr [rsi + 8]
	mov r13, qword ptr [rsi + 16]
	mov rax, r15
	shr rax, 60
	jne .LBB0_8
	lea rcx, [8*r15]
	mov rdx, qword ptr [rbx]
	mov rax, qword ptr [rdx]
	mov rsi, qword ptr [rdx + 8]
	add rax, 7
	and rax, -8
	sub rsi, rax
	cmp rcx, rsi
	ja .LBB0_0
	add rcx, rax
	mov qword ptr [rdx], rcx
	test rax, rax
	jne .LBB0_1
.LBB0_0:
	mov rdi, rbx
	mov rsi, r15
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_slice_in_another_chunk
	test rax, rax
	je .LBB0_8
.LBB0_1:
	test r15, r15
	je .LBB0_6
	movabs rdi, 2305843009213693951
	lea rcx, [r15 - 1]
	and rcx, rdi
	cmp r15, rcx
	cmovb rcx, r15
	mov rdx, r12
	mov rsi, rax
	cmp rcx, 33
	jbe .LBB0_4
	lea rdx, [r15 + rdi]
	and rdx, rdi
	cmp r15, rdx
	cmovb rdx, r15
	lea rsi, [r12 + 4*rdx]
	add rsi, 4
	cmp rax, rsi
	jae .LBB0_2
	lea rdi, [rax + 8*rdx]
	add rdi, 8
	mov rdx, r12
	mov rsi, rax
	cmp r12, rdi
	jb .LBB0_4
.LBB0_2:
	inc rcx
	mov edx, ecx
	and edx, 3
	mov esi, 4
	cmovne rsi, rdx
	sub rcx, rsi
	lea rdx, [r12 + 4*rcx]
	lea rsi, [rax + 8*rcx]
	xor edi, edi
	xorps xmm0, xmm0
.LBB0_3:
	movsd xmm1, qword ptr [r12 + 4*rdi]
	movsd xmm2, qword ptr [r12 + 4*rdi + 8]
	unpcklps xmm1, xmm0
	unpcklps xmm2, xmm0
	movups xmmword ptr [rax + 8*rdi], xmm1
	movups xmmword ptr [rax + 8*rdi + 16], xmm2
	add rdi, 4
	cmp rcx, rdi
	jne .LBB0_3
.LBB0_4:
	lea rcx, [r12 + 4*r15]
	lea rdi, [rax + 8*r15]
.LBB0_5:
	cmp rdx, rcx
	je .LBB0_11
	mov r8d, dword ptr [rdx]
	add rdx, 4
	mov qword ptr [rsi], r8
	add rsi, 8
	cmp rsi, rdi
	jne .LBB0_5
.LBB0_6:
	movabs rcx, 4611686018427387903
	and r15, rcx
	lea rdx, [r12 + 4*r13]
	mov rcx, qword ptr [rbx]
	cmp rdx, qword ptr [rcx]
	jne .LBB0_7
	mov qword ptr [rcx], r12
.LBB0_7:
	mov qword ptr [r14], rax
	mov qword ptr [r14 + 8], r15
	mov qword ptr [r14 + 16], r15
	mov qword ptr [r14 + 24], rbx
	jmp .LBB0_10
.LBB0_8:
	lea rcx, [r12 + 4*r13]
	mov rax, qword ptr [rbx]
	cmp rcx, qword ptr [rax]
	jne .LBB0_9
	mov qword ptr [rax], r12
.LBB0_9:
	mov qword ptr [r14], 0
.LBB0_10:
	mov rax, r14
	pop rbx
	pop r12
	pop r13
	pop r14
	pop r15
	ret
.LBB0_11:
	call qword ptr [rip + bump_scope::exact_size_iterator_bad_len@GOTPCREL]
	ud2
	jmp .LBB0_12
.LBB0_12:
	lea rdx, [r12 + 4*r13]
	mov rcx, qword ptr [rbx]
	cmp rdx, qword ptr [rcx]
	jne .LBB0_13
	mov qword ptr [rcx], r12
.LBB0_13:
	mov rdi, rax
	call _Unwind_Resume@PLT

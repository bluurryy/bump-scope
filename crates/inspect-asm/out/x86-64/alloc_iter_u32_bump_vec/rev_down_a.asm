inspect_asm::alloc_iter_u32_bump_vec::rev_down_a:
	push rbp
	push r15
	push r14
	push r12
	push rbx
	sub rsp, 32
	mov qword ptr [rsp], 4
	xorps xmm0, xmm0
	movups xmmword ptr [rsp + 16], xmm0
	mov qword ptr [rsp + 8], rdi
	mov eax, 4
	test rdx, rdx
	jne .LBB0_1
	xor edx, edx
.LBB0_0:
	add rsp, 32
	pop rbx
	pop r12
	pop r14
	pop r15
	pop rbp
	ret
.LBB0_1:
	mov rbx, rsp
	mov rdi, rbx
	mov r14, rsi
	mov rsi, rdx
	mov r15, rdx
	call bump_scope::mut_bump_vec_rev::MutBumpVecRev<T,A>::generic_grow_amortized
	mov rcx, r14
	mov rax, r15
	mov rdx, qword ptr [rsp + 16]
	shl rax, 2
	xor r15d, r15d
	jmp .LBB0_3
.LBB0_2:
	mov rsi, qword ptr [rsp]
	mov rdi, rdx
	not rdi
	mov dword ptr [rsi + 4*rdi], ebp
	inc rdx
	mov qword ptr [rsp + 16], rdx
	add r15, 4
	cmp rax, r15
	je .LBB0_4
.LBB0_3:
	mov ebp, dword ptr [rcx + r15]
	cmp qword ptr [rsp + 24], rdx
	jne .LBB0_2
	mov esi, 1
	mov rdi, rbx
	mov r12, rax
	call bump_scope::mut_bump_vec_rev::MutBumpVecRev<T,A>::generic_grow_amortized
	mov rcx, r14
	mov rax, r12
	mov rdx, qword ptr [rsp + 16]
	jmp .LBB0_2
.LBB0_4:
	cmp qword ptr [rsp + 24], 0
	je .LBB0_5
	mov rcx, qword ptr [rsp + 8]
	lea rax, [rsi + 4*rdi]
	mov rcx, qword ptr [rcx]
	mov qword ptr [rcx], rax
	jmp .LBB0_0
.LBB0_5:
	xor edx, edx
	mov eax, 4
	jmp .LBB0_0
